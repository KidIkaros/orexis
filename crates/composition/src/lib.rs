//! The composition root: the only place that constructs concrete adapters.

use std::sync::Arc;

use orexis_adapters_fake::{EchoTools, InMemorySession, RecordingSink, ScriptedProvider};
use orexis_core::Action;
use orexis_ports::{Provider, SessionAppend, SessionRead, ToolExecutor};

pub struct App {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub session: Arc<dyn SessionAppend>,
    pub reader: Arc<dyn SessionRead>,
    pub sink: Arc<RecordingSink>,
}

pub fn build() -> App {
    let provider = Arc::new(ScriptedProvider::new(vec![
        vec![
            Action::Say {
                text: "reading the file".to_string(),
            },
            Action::CallTool {
                call_id: "call-1".to_string(),
                name: "read".to_string(),
                args: "src/lib.rs".to_string(),
            },
        ],
        vec![Action::Done {
            summary: "read and summarized".to_string(),
        }],
    ]));

    let session = Arc::new(InMemorySession::default());
    let session_append: Arc<dyn SessionAppend> = session.clone();
    let session_read: Arc<dyn SessionRead> = session;

    App {
        provider,
        tools: Arc::new(EchoTools),
        session: session_append,
        reader: session_read,
        sink: Arc::new(RecordingSink::default()),
    }
}
