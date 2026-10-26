//! A skeleton `serve` binary. M1-08 wires the real provider catalog and tools.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{EchoTools, ScriptedProvider};
use orexis_core::Action;
use orexis_daemon::{Daemon, DaemonConfig};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let socket = std::env::var("OREXIS_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.sock"));
    let db = std::env::var("OREXIS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.db"));

    let provider = Arc::new(ScriptedProvider::new(vec![vec![Action::Done {
        summary: "fake provider; configure a real one in M1-08".to_string(),
    }]]));

    let config = DaemonConfig {
        socket,
        db,
        provider,
        tools: Arc::new(EchoTools),
        codec: Arc::new(NativeCodec),
        system_prompt: String::new(),
        context_tools: Vec::new(),
        context_files: Vec::new(),
        context_limit: 32768,
        max_steps: 8,
        generation_slots: 1,
    };

    Daemon::new(config).run().await
}
