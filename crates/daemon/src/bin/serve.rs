//! `serve`: build the runtime from the catalog and run the daemon.
//! `OREXIS_FAKE=1` uses deterministic fakes for tests.

use orexis_daemon::{daemon_config_from_env, Daemon};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = daemon_config_from_env()?;
    Daemon::new(config).run().await?;
    Ok(())
}
