//! Compat entry point for `serve`; the user-facing binary is `orexis
//! serve` (ticket 019). Kept for the daemon tests' `CARGO_BIN_EXE_serve`.

use orexis_daemon::serve_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_main().await
}
