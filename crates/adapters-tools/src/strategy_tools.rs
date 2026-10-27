//! The strategy toolset filter over any tool executor (M8-05, ADR-0020):
//! a worker's toolset can only NARROW the wrapped executor's verdicts —
//! a tool outside the strategy's allowlist is refused before the gate is
//! even asked, and the gate's own verdicts are untouched. The floor
//! survives YOLO.

use std::sync::Arc;

use async_trait::async_trait;
use orexis_core::{Observation, ToolsetFilter};
use orexis_ports::{PortError, PortResult, ToolExecutor};

pub struct StrategyTools {
    inner: Arc<dyn ToolExecutor>,
    filter: ToolsetFilter,
}

impl StrategyTools {
    pub fn new(inner: Arc<dyn ToolExecutor>, filter: ToolsetFilter) -> Self {
        Self { inner, filter }
    }
}

#[async_trait]
impl ToolExecutor for StrategyTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        if !self.filter.permits(name) {
            return Err(PortError(format!(
                "tool {name} is outside this strategy's toolset"
            )));
        }
        self.inner.execute(name, args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoExecutor;

    #[async_trait]
    impl ToolExecutor for EchoExecutor {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: "ran".to_string(),
            })
        }
    }

    fn tools(filter: ToolsetFilter) -> StrategyTools {
        StrategyTools::new(Arc::new(EchoExecutor), filter)
    }

    #[tokio::test]
    async fn the_filter_narrows() {
        let tools = tools(ToolsetFilter::allow(&["read_file", "mcp"]));
        assert!(tools.execute("read_file", "{}").await.is_ok());
        assert!(tools.execute("mcp/server/tool", "{}").await.is_ok());
        let error = tools
            .execute("write_file", "{}")
            .await
            .expect_err("narrowed");
        assert!(error.0.contains("outside this strategy's toolset"));
    }

    #[tokio::test]
    async fn an_open_filter_passes_everything_through() {
        let tools = tools(ToolsetFilter::open());
        assert!(tools.execute("write_file", "{}").await.is_ok());
    }
}
