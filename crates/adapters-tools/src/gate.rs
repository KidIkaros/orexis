//! The permission gate over any tool executor.

use std::sync::Arc;

use async_trait::async_trait;
use orexis_core::{Decision, Observation, PermissionPolicy};
use orexis_ports::{ApprovalGate, PortError, PortResult, ToolExecutor};

/// Wraps a tool executor with the permission ruleset and an approval gate.
pub struct GatedTools {
    inner: Arc<dyn ToolExecutor>,
    policy: Arc<PermissionPolicy>,
    gate: Arc<dyn ApprovalGate>,
}

impl GatedTools {
    pub fn new(
        inner: Arc<dyn ToolExecutor>,
        policy: Arc<PermissionPolicy>,
        gate: Arc<dyn ApprovalGate>,
    ) -> Self {
        Self {
            inner,
            policy,
            gate,
        }
    }
}

#[async_trait]
impl ToolExecutor for GatedTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        match self.policy.decide(name) {
            Decision::Allow => self.inner.execute(name, args).await,
            Decision::Deny => Err(PortError(format!("denied by policy: {name}"))),
            Decision::Ask => {
                if self.gate.approve(name, args).await? {
                    self.inner.execute(name, args).await
                } else {
                    Err(PortError(format!("approval denied: {name}")))
                }
            }
        }
    }
}

pub struct AutoApprove;

#[async_trait]
impl ApprovalGate for AutoApprove {
    async fn approve(&self, _tool: &str, _args: &str) -> PortResult<bool> {
        Ok(true)
    }
}

pub struct DenyAll;

#[async_trait]
impl ApprovalGate for DenyAll {
    async fn approve(&self, _tool: &str, _args: &str) -> PortResult<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use futures::executor::block_on;

    struct StubTools;

    #[async_trait]
    impl ToolExecutor for StubTools {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: "ran".to_string(),
            })
        }
    }

    fn gated(policy: PermissionPolicy, gate: Arc<dyn ApprovalGate>) -> GatedTools {
        GatedTools::new(Arc::new(StubTools), Arc::new(policy), gate)
    }

    #[test]
    fn allow_passes() {
        let tools = gated(PermissionPolicy::standard(), Arc::new(AutoApprove));
        let observation = block_on(tools.execute("read_file", "{}")).expect("allow");
        assert_eq!(observation.output, "ran");
    }

    #[test]
    fn deny_blocks() {
        let tools = gated(PermissionPolicy::plan(), Arc::new(AutoApprove));
        assert!(block_on(tools.execute("write_file", "{}")).is_err());
    }

    #[test]
    fn ask_uses_the_gate() {
        let tools = gated(PermissionPolicy::standard(), Arc::new(AutoApprove));
        assert!(block_on(tools.execute("shell", "{}")).is_ok());

        let tools = gated(PermissionPolicy::standard(), Arc::new(DenyAll));
        assert!(block_on(tools.execute("shell", "{}")).is_err());
    }
}
