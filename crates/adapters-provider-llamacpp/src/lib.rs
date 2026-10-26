//! llama.cpp process supervision. The wire is OpenAI-compatible, so the
//! provider itself is `OpenAiCompatibleProvider` with the llamacpp profile.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("spawn: {0}")]
    Spawn(#[from] std::io::Error),
}

pub struct LlamaCppServer {
    binary: PathBuf,
    model: PathBuf,
    port: u16,
    child: Option<Child>,
}

impl LlamaCppServer {
    pub fn new(binary: impl Into<PathBuf>, model: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            binary: binary.into(),
            model: model.into(),
            port,
            child: None,
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }

    pub fn command_args(&self) -> Vec<String> {
        vec![
            "-m".to_string(),
            self.model.display().to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            self.port.to_string(),
        ]
    }

    pub fn start(&mut self) -> Result<(), ServerError> {
        let mut command = Command::new(&self.binary);
        command
            .args(self.command_args())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        self.child = Some(command.spawn()?);
        Ok(())
    }

    pub fn health(&self) -> bool {
        let address = SocketAddr::from(([127, 0, 0, 1], self.port));
        TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok()
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for LlamaCppServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_args_include_model_and_port() {
        let server = LlamaCppServer::new("/bin/true", "/tmp/model.gguf", 8081);
        let args = server.command_args();
        assert_eq!(args[0], "-m");
        assert_eq!(args[1], "/tmp/model.gguf");
        assert!(args.contains(&"--port".to_string()));
        assert!(args.contains(&"8081".to_string()));
    }

    #[test]
    fn base_url_is_openai_compatible() {
        let server = LlamaCppServer::new("/bin/true", "/tmp/model.gguf", 8081);
        assert_eq!(server.base_url(), "http://127.0.0.1:8081/v1");
    }

    #[test]
    fn stop_is_idempotent() {
        let mut server = LlamaCppServer::new("/bin/true", "/tmp/model.gguf", 8081);
        server.stop();
        server.stop();
        assert!(!server.is_running());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn start_and_stop_a_child() {
        let mut server = LlamaCppServer::new("/bin/true", "/tmp/model.gguf", 8081);
        server.start().expect("spawn");
        assert!(server.is_running());
        server.stop();
        assert!(!server.is_running());
    }
}
