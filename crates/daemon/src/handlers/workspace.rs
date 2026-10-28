//! Workspace handlers: list, read, artifacts.

use std::sync::Arc;

use orexis_ports::Workspace;
use tokio::net::unix::OwnedWriteHalf;

use crate::proto::{write_msg, Artifact, FileEntry, ServerMsg};
use crate::server::DaemonConfig;
use crate::session::SessionRuntime;

use super::session::latest_checkpoint;

pub(crate) async fn list_dir(
    workspace: &Arc<dyn Workspace>,
    path: String,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let entries = match workspace.list(&path) {
        Ok(entries) => entries
            .into_iter()
            .map(|entry| FileEntry {
                name: entry.name,
                path: entry.path,
                is_dir: entry.is_dir,
                size: entry.size,
            })
            .collect(),
        Err(error) => {
            write_msg(
                write,
                &ServerMsg::Error {
                    message: error.to_string(),
                },
            )
            .await?;
            return Ok(());
        }
    };
    write_msg(write, &ServerMsg::Dir { path, entries }).await
}

pub(crate) async fn read_file(
    workspace: &Arc<dyn Workspace>,
    path: String,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let preview = match workspace.read(&path, 64 * 1024) {
        Ok(preview) => preview,
        Err(error) => {
            write_msg(
                write,
                &ServerMsg::Error {
                    message: error.to_string(),
                },
            )
            .await?;
            return Ok(());
        }
    };
    write_msg(
        write,
        &ServerMsg::File {
            path: preview.path,
            content: preview.content,
            truncated: preview.truncated,
        },
    )
    .await
}

/// Team mode reads the blackboard — the real artifact store (M8-11);
/// off, the session projection (changed files plus a checkpoint diff).
pub(crate) async fn artifacts(
    rt: &Arc<SessionRuntime>,
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    if config.team {
        let items = match config.blackboard.list().await {
            Ok(names) => names
                .into_iter()
                .map(|name| Artifact {
                    id: name.clone(),
                    kind: "artifact".to_string(),
                    title: name,
                    detail: String::new(),
                })
                .collect(),
            Err(error) => {
                return write_msg(
                    write,
                    &ServerMsg::Error {
                        message: error.to_string(),
                    },
                )
                .await
            }
        };
        return write_msg(write, &ServerMsg::Artifacts { items }).await;
    }
    let changed = match config.workspace.changed() {
        Ok(changed) => changed,
        Err(error) => {
            write_msg(
                write,
                &ServerMsg::Error {
                    message: error.to_string(),
                },
            )
            .await?;
            return Ok(());
        }
    };
    let mut items = Vec::new();
    for path in changed {
        items.push(Artifact {
            id: path.clone(),
            kind: "file".to_string(),
            title: path,
            detail: String::new(),
        });
    }
    if let Some(id) = latest_checkpoint(rt).await? {
        if let Some(checkpointer) = &config.checkpointer {
            if let Ok(diff) = checkpointer.diff(&id) {
                items.push(Artifact {
                    id: format!("checkpoint:{id}"),
                    kind: "diff".to_string(),
                    title: format!("checkpoint {id}"),
                    detail: diff,
                });
            }
        }
    }
    write_msg(write, &ServerMsg::Artifacts { items }).await
}
