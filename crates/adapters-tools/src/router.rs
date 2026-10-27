//! Route tool calls between built-in and external executors by namespace.

use std::sync::Arc;

use async_trait::async_trait;
use orexis_core::{is_external_tool, Observation};
use orexis_ports::{ExternalTools, PortError, PortResult, ToolExecutor};

/// Dispatches `mcp/…` and `utcp/…` names to the external executor and
/// everything else to the built-in one. One tool surface, two arms.
pub struct RouterTools {
    built_in: Arc<dyn ToolExecutor>,
    external: Option<Arc<dyn ToolExecutor>>,
}

impl RouterTools {
    pub fn new(built_in: Arc<dyn ToolExecutor>, external: Option<Arc<dyn ToolExecutor>>) -> Self {
        Self { built_in, external }
    }
}

#[async_trait]
impl ToolExecutor for RouterTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        if is_external_tool(name) {
            let external = self.external.as_ref().ok_or_else(|| {
                PortError(format!("no external tool source is configured: {name}"))
            })?;
            external.execute(name, args).await
        } else {
            self.built_in.execute(name, args).await
        }
    }
}

/// Fans namespaced calls out to the registered external sources by the
/// namespace each one serves (`mcp/`, `utcp/`). Composition builds it
/// from the configured `ExternalTools` sources; it sits behind
/// `RouterTools`' external arm so the tool surface stays one executor.
pub struct NamespaceTools {
    arms: Vec<(String, Arc<dyn ToolExecutor>)>,
}

impl NamespaceTools {
    pub fn from_sources(sources: &[Arc<dyn ExternalTools>]) -> Self {
        Self {
            arms: sources
                .iter()
                .map(|source| {
                    (
                        format!("{}/", source.kind()),
                        source.clone() as Arc<dyn ToolExecutor>,
                    )
                })
                .collect(),
        }
    }
}

#[async_trait]
impl ToolExecutor for NamespaceTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        for (prefix, arm) in &self.arms {
            if name.starts_with(prefix.as_str()) {
                return arm.execute(name, args).await;
            }
        }
        Err(PortError(format!("no external tool source serves: {name}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    struct StubTools {
        tag: &'static str,
    }

    #[async_trait]
    impl ToolExecutor for StubTools {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: self.tag.to_string(),
            })
        }
    }

    #[test]
    fn built_in_names_stay_built_in() {
        let router = RouterTools::new(
            Arc::new(StubTools { tag: "local" }),
            Some(Arc::new(StubTools { tag: "external" })),
        );
        assert_eq!(
            block_on(router.execute("read_file", "{}"))
                .expect("ok")
                .output,
            "local"
        );
        assert_eq!(
            block_on(router.execute("shell", "{}")).expect("ok").output,
            "local"
        );
    }

    #[test]
    fn namespaced_names_go_external() {
        let router = RouterTools::new(
            Arc::new(StubTools { tag: "local" }),
            Some(Arc::new(StubTools { tag: "external" })),
        );
        assert_eq!(
            block_on(router.execute("mcp/fs/read_file", "{}"))
                .expect("ok")
                .output,
            "external"
        );
        assert_eq!(
            block_on(router.execute("utcp/http/call", "{}"))
                .expect("ok")
                .output,
            "external"
        );
    }

    #[test]
    fn namespaced_names_without_a_source_error() {
        let router = RouterTools::new(Arc::new(StubTools { tag: "local" }), None);
        assert!(block_on(router.execute("mcp/fs/read_file", "{}")).is_err());
    }

    #[test]
    fn lookalike_names_stay_built_in() {
        let router = RouterTools::new(
            Arc::new(StubTools { tag: "local" }),
            Some(Arc::new(StubTools { tag: "external" })),
        );
        assert_eq!(
            block_on(router.execute("mcpx/read_file", "{}"))
                .expect("ok")
                .output,
            "local"
        );
    }

    struct FakeSource {
        kind: &'static str,
        tag: &'static str,
    }

    #[async_trait]
    impl ToolExecutor for FakeSource {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: self.tag.to_string(),
            })
        }
    }

    #[async_trait]
    impl ExternalTools for FakeSource {
        fn kind(&self) -> &'static str {
            self.kind
        }

        async fn connect_all(&self) -> Vec<(String, String)> {
            Vec::new()
        }

        async fn specs(&self) -> Vec<orexis_core::ToolSpec> {
            Vec::new()
        }
    }

    #[test]
    fn fanout_routes_each_namespace_to_its_source() {
        let sources: Vec<Arc<dyn ExternalTools>> = vec![
            Arc::new(FakeSource {
                kind: "mcp",
                tag: "mcp-arm",
            }),
            Arc::new(FakeSource {
                kind: "utcp",
                tag: "utcp-arm",
            }),
        ];
        let fanout = NamespaceTools::from_sources(&sources);
        assert_eq!(
            block_on(fanout.execute("mcp/fs/read", "{}"))
                .expect("ok")
                .output,
            "mcp-arm"
        );
        assert_eq!(
            block_on(fanout.execute("utcp/http/ping", "{}"))
                .expect("ok")
                .output,
            "utcp-arm"
        );
        assert!(block_on(fanout.execute("a2a/peer/send", "{}")).is_err());
    }
}
