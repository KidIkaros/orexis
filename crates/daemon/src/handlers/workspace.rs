//! Workspace handlers: list, read, artifacts.

use std::sync::Arc;

use orexis_ports::{Checkpointer, Workspace};
use tokio::net::unix::OwnedWriteHalf;

use crate::proto::{write_msg, Artifact, FileEntry, ServerMsg};
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

pub(crate) async fn artifacts(
    rt: &Arc<SessionRuntime>,
    workspace: &Arc<dyn Workspace>,
    checkpointer: Option<&Arc<dyn Checkpointer>>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let changed = match workspace.changed() {
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
        if let Some(checkpointer) = checkpointer {
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
