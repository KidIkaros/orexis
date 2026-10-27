//! The subcommand bodies shared by the compat bins and the `orexis`
//! binary (ticket 019): one implementation, two entry points.

use crate::{daemon_config_from_env, AcpServer, Daemon};

/// `orexis serve` — build the runtime from the environment and run the
/// daemon. `OREXIS_FAKE=1` uses deterministic fakes for tests.
pub async fn serve_main() -> Result<(), Box<dyn std::error::Error>> {
    let config = daemon_config_from_env()?;
    Daemon::new(config).run().await?;
    Ok(())
}

/// `orexis acp` — serve Orexis as an ACP agent over stdio (M7-06).
pub async fn acp_main() -> Result<(), Box<dyn std::error::Error>> {
    let config = daemon_config_from_env()?;
    let server = AcpServer::new(config);
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    server.run(&mut reader, tokio::io::stdout()).await?;
    Ok(())
}
