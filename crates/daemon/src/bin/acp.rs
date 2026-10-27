//! Compat entry point for `acp`; the user-facing binary is `orexis acp`
//! (ticket 019).

use orexis_daemon::acp_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    acp_main().await
}
