//! The board store and its port.

use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use orexis_core::{BoardTask, TaskStatus};
use orexis_ports::{BoardStore, PortError, PortResult};
use rusqlite::{params, Connection, OptionalExtension};

use crate::migrate::migrate;
use crate::util::{err, lock, new_id};

/// The board's default task cap (§9: 256; a team may tighten, never
/// exceed).
pub const DEFAULT_TASK_CAP: usize = 256;

pub struct SqliteBoard {
    conn: Mutex<Connection>,
    cap: usize,
}

impl SqliteBoard {
    pub fn open(path: &Path) -> PortResult<Self> {
        Self::open_with_cap(path, DEFAULT_TASK_CAP)
    }

    pub fn open_with_cap(path: &Path, cap: usize) -> PortResult<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            cap,
        })
    }

    fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<BoardTask> {
        Ok(BoardTask {
            id: row.get(0)?,
            title: row.get(1)?,
            required_strategy: row.get(2)?,
            owner: row.get(3)?,
            status: status_of(&row.get::<_, String>(4)?),
            depends_on: Vec::new(),
            seq: row.get(5)?,
        })
    }
}

fn status_of(text: &str) -> TaskStatus {
    match text {
        "admitted" => TaskStatus::Admitted,
        "running" => TaskStatus::Running,
        "blocked" => TaskStatus::Blocked,
        "done" => TaskStatus::Done,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Queued,
    }
}

#[async_trait]
impl BoardStore for SqliteBoard {
    async fn create(
        &self,
        title: &str,
        required_strategy: &str,
        depends_on: Vec<String>,
    ) -> PortResult<BoardTask> {
        let conn = self.conn.lock().map_err(lock)?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .map_err(err)?;
        if count as usize >= self.cap {
            return Err(PortError(format!("the board is full ({} tasks)", self.cap)));
        }
        for edge in &depends_on {
            let known: Option<String> = conn
                .query_row("SELECT id FROM task WHERE id = ?1", params![edge], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(err)?;
            if known.is_none() {
                return Err(PortError(format!("unknown task in depends_on: {edge}")));
            }
        }
        let id = new_id();
        conn.execute(
            "INSERT INTO task (id, title, required_strategy, owner, status, seq, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 'queued',
                     COALESCE((SELECT MAX(seq) FROM task), 0) + 1, unixepoch(), unixepoch())",
            params![id, title, required_strategy],
        )
        .map_err(err)?;
        for edge in &depends_on {
            conn.execute(
                "INSERT INTO task_edge (task_id, depends_on) VALUES (?1, ?2)",
                params![id, edge],
            )
            .map_err(err)?;
        }
        Ok(BoardTask {
            id,
            title: title.to_string(),
            required_strategy: required_strategy.to_string(),
            owner: None,
            status: TaskStatus::Queued,
            depends_on,
            seq: count as u64 + 1,
        })
    }

    async fn get(&self, id: &str) -> PortResult<Option<BoardTask>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut task = conn
            .query_row(
                "SELECT id, title, required_strategy, owner, status, seq FROM task WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .optional()
            .map_err(err)?;
        if let Some(task) = task.as_mut() {
            task.depends_on = edges_of(&conn, &task.id)?;
        }
        Ok(task)
    }

    async fn list(&self) -> PortResult<Vec<BoardTask>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut statement = conn
            .prepare(
                "SELECT id, title, required_strategy, owner, status, seq FROM task ORDER BY seq",
            )
            .map_err(err)?;
        let mut tasks = statement
            .query_map([], Self::row_to_task)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        for task in &mut tasks {
            task.depends_on = edges_of(&conn, &task.id)?;
        }
        Ok(tasks)
    }

    async fn ready(&self) -> PortResult<Vec<BoardTask>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut statement = conn
            .prepare(
                "SELECT id, title, required_strategy, owner, status, seq FROM task
                 WHERE status = 'queued'
                   AND NOT EXISTS (
                     SELECT 1 FROM task_edge edge
                     JOIN task dep ON dep.id = edge.depends_on
                     WHERE edge.task_id = task.id AND dep.status != 'done'
                   )
                 ORDER BY seq",
            )
            .map_err(err)?;
        let mut tasks = statement
            .query_map([], Self::row_to_task)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        for task in &mut tasks {
            task.depends_on = edges_of(&conn, &task.id)?;
        }
        Ok(tasks)
    }

    async fn admit(&self, id: &str) -> PortResult<BoardTask> {
        self.transition(id, "queued", "admitted", None).await
    }

    async fn start(&self, id: &str, owner: &str) -> PortResult<BoardTask> {
        self.transition(id, "admitted", "running", Some(owner))
            .await
    }

    async fn complete(&self, id: &str) -> PortResult<BoardTask> {
        self.transition(id, "running", "done", None).await
    }

    async fn fail(&self, id: &str, reason: &str) -> PortResult<BoardTask> {
        let conn = self.conn.lock().map_err(lock)?;
        let changed = conn
            .execute(
                "UPDATE task SET status = 'failed', title = title || ' — failed: ' || ?2,
                 updated_at = unixepoch() WHERE id = ?1",
                params![id, reason],
            )
            .map_err(err)?;
        if changed == 0 {
            return Err(PortError(format!("no task {id}")));
        }
        conn.query_row(
            "SELECT id, title, required_strategy, owner, status, seq FROM task WHERE id = ?1",
            params![id],
            Self::row_to_task,
        )
        .map_err(err)
    }
}

impl SqliteBoard {
    /// A guarded transition: the task must be in `from`; `owner` is set
    /// when the transition starts a worker.
    async fn transition(
        &self,
        id: &str,
        from: &str,
        to: &str,
        owner: Option<&str>,
    ) -> PortResult<BoardTask> {
        let conn = self.conn.lock().map_err(lock)?;
        let changed = match owner {
            Some(owner) => conn
                .execute(
                    "UPDATE task SET status = ?2, owner = ?3, updated_at = unixepoch()
                     WHERE id = ?1 AND status = ?4",
                    params![id, to, owner, from],
                )
                .map_err(err)?,
            None => conn
                .execute(
                    "UPDATE task SET status = ?2, updated_at = unixepoch()
                     WHERE id = ?1 AND status = ?3",
                    params![id, to, from],
                )
                .map_err(err)?,
        };
        if changed == 0 {
            let status: Option<String> = conn
                .query_row(
                    "SELECT status FROM task WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(err)?;
            return Err(match status {
                None => PortError(format!("no task {id}")),
                Some(actual) => PortError(format!("task {id} is {actual}, not {from}")),
            });
        }
        let mut task = conn
            .query_row(
                "SELECT id, title, required_strategy, owner, status, seq FROM task WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .map_err(err)?;
        task.depends_on = edges_of(&conn, id)?;
        Ok(task)
    }
}

fn edges_of(conn: &Connection, id: &str) -> PortResult<Vec<String>> {
    let mut statement = conn
        .prepare("SELECT depends_on FROM task_edge WHERE task_id = ?1 ORDER BY rowid")
        .map_err(err)?;
    let rows = statement
        .query_map(params![id], |row| row.get(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err);
    drop(statement);
    rows
}
