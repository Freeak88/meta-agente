use crate::state::{AgentStateSnapshot, State};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub blacklisted_capabilities: HashSet<String>,
    pub max_consecutive_failures: u32,
    pub max_total_executions: u32,
}

impl RiskConfig {
    pub fn default() -> Self {
        Self {
            blacklisted_capabilities: HashSet::new(),
            max_consecutive_failures: 5,
            max_total_executions: 100,
        }
    }

    pub fn strict() -> Self {
        let mut blacklist = HashSet::new();
        blacklist.insert("http_delete".to_string());
        blacklist.insert("db_drop".to_string());

        Self {
            blacklisted_capabilities: blacklist,
            max_consecutive_failures: 3,
            max_total_executions: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskDecision {
    Allow,
    Block { reason: String },
}

pub struct RiskGate {
    config: RiskConfig,
}

pub trait RiskContext {
    fn consecutive_failures(&self) -> u32;
    fn total_executions(&self) -> u32;
    fn is_paused(&self) -> bool;
}

impl RiskContext for State {
    fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    fn total_executions(&self) -> u32 {
        self.total_executions
    }

    fn is_paused(&self) -> bool {
        self.paused
    }
}

impl RiskContext for AgentStateSnapshot {
    fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    fn total_executions(&self) -> u32 {
        self.total_executions
    }

    fn is_paused(&self) -> bool {
        self.paused
    }
}

impl RiskGate {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }

    pub fn evaluate<C: RiskContext>(&self, capability: &str, context: &C) -> RiskDecision {
        if self.config.blacklisted_capabilities.contains(capability) {
            return RiskDecision::Block {
                reason: format!("capability_blacklisted: {}", capability),
            };
        }

        if context.consecutive_failures() >= self.config.max_consecutive_failures {
            return RiskDecision::Block {
                reason: format!(
                    "max_consecutive_failures_exceeded: {} >= {}",
                    context.consecutive_failures(),
                    self.config.max_consecutive_failures
                ),
            };
        }

        if context.total_executions() >= self.config.max_total_executions {
            return RiskDecision::Block {
                reason: format!(
                    "max_total_executions_exceeded: {} >= {}",
                    context.total_executions(),
                    self.config.max_total_executions
                ),
            };
        }

        if context.is_paused() {
            return RiskDecision::Block {
                reason: "state_already_paused".to_string(),
            };
        }

        RiskDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_normal_capability() {
        let gate = RiskGate::new(RiskConfig::default());
        let state = State::new("test");

        let decision = gate.evaluate("http_ping", &state);

        assert_eq!(decision, RiskDecision::Allow);
    }

    #[test]
    fn blocks_blacklisted_capability() {
        let mut config = RiskConfig::default();
        config
            .blacklisted_capabilities
            .insert("http_delete".to_string());
        let gate = RiskGate::new(config);
        let state = State::new("test");

        match gate.evaluate("http_delete", &state) {
            RiskDecision::Block { reason } => assert!(reason.contains("blacklisted")),
            RiskDecision::Allow => panic!("expected Block"),
        }
    }

    #[test]
    fn blocks_too_many_failures() {
        let gate = RiskGate::new(RiskConfig::default());
        let mut state = State::new("test");
        state.consecutive_failures = 5;

        match gate.evaluate("http_ping", &state) {
            RiskDecision::Block { reason } => assert!(reason.contains("consecutive_failures")),
            RiskDecision::Allow => panic!("expected Block"),
        }
    }

    #[test]
    fn blocks_too_many_executions() {
        let gate = RiskGate::new(RiskConfig::default());
        let mut state = State::new("test");
        state.total_executions = 100;

        match gate.evaluate("http_ping", &state) {
            RiskDecision::Block { reason } => assert!(reason.contains("total_executions")),
            RiskDecision::Allow => panic!("expected Block"),
        }
    }

    #[test]
    fn strict_config_blocks_destructive_capabilities() {
        let gate = RiskGate::new(RiskConfig::strict());
        let state = State::new("test");

        match gate.evaluate("db_drop", &state) {
            RiskDecision::Block { reason } => assert!(reason.contains("db_drop")),
            RiskDecision::Allow => panic!("expected Block"),
        }
    }

    #[test]
    fn evaluates_agent_state_snapshot() {
        let gate = RiskGate::new(RiskConfig::default());
        let mut state = State::new("test_snapshot_risk");
        state.total_executions = 100;
        let snapshot = state.snapshot();

        match gate.evaluate("http_ping", &snapshot) {
            RiskDecision::Block { reason } => assert!(reason.contains("total_executions")),
            RiskDecision::Allow => panic!("expected Block"),
        }
    }
}
