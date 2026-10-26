//! `acp`: serve Orexis as an ACP agent over stdio (M7-06).
//! `OREXIS_FAKE=1` uses deterministic fakes for tests.

use orexis_daemon::{daemon_config_from_env, AcpServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = daemon_config_from_env()?;
    let server = AcpServer::new(config);
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    server.run(&mut reader, tokio::io::stdout()).await?;
    Ok(())
}
