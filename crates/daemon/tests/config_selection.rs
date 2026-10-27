//! Integration: the selection `orexis init` writes is what `serve` builds
//! from when the environment is silent (ticket 014). Spawned processes
//! keep the env manipulation race-free.

use std::path::Path;
use std::time::Duration;

use tokio::net::UnixStream;
use tokio::process::Command;

fn serve(socket: &Path, db: &Path, project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_serve"));
    command
        .env("OREXIS_SOCKET", socket)
        .env("OREXIS_DB", db)
        .env("OREXIS_PROJECT", project)
        .env_remove("OREXIS_PROFILE")
        .env_remove("OREXIS_MODEL")
        .env_remove("OREXIS_FAKE")
        .kill_on_drop(true);
    command
}

#[tokio::test]
async fn the_written_selection_is_what_serve_builds_from() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    // A selection naming a profile the catalog does not ship: if serve
    // reads the config, startup fails naming it; if it fell back to a
    // silent default instead, startup would succeed and the test would
    // fail.
    orexis_composition::write_selection(
        dir.path(),
        &orexis_composition::Selection {
            profile_id: "no-such-profile".to_string(),
            model_id: "no-such-model".to_string(),
        },
    )
    .expect("write selection");

    let mut child = serve(&socket, &db, dir.path())
        .spawn()
        .expect("spawn serve");
    let output = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("serve should exit on an unknown profile")
        .expect("wait");
    assert!(
        !output.success(),
        "an unknown profile in the selection must fail startup"
    );
}

#[tokio::test]
async fn the_environment_overrides_the_written_selection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    orexis_composition::write_selection(
        dir.path(),
        &orexis_composition::Selection {
            profile_id: "no-such-profile".to_string(),
            model_id: "no-such-model".to_string(),
        },
    )
    .expect("write selection");

    // The env names a real profile: the broken selection must be ignored.
    let mut command = serve(&socket, &db, dir.path());
    command
        .env("OREXIS_PROFILE", "openai")
        .env("OREXIS_MODEL", "gpt-5");
    let mut child = command.spawn().expect("spawn serve");

    // Connect: the daemon started, so the env won.
    let mut connected = false;
    for _ in 0..300 {
        if UnixStream::connect(&socket).await.is_ok() {
            connected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    child.kill().await.ok();
    assert!(connected, "the daemon must start on the env's profile");
}
