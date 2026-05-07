use crate::capability::{CapabilityConfig, CapabilityRegistry, ExecutionResult};
use crate::risk::{RiskConfig, RiskDecision, RiskGate};
use crate::state::{AgentStateSnapshot, State, StepResult};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub enum InputSource {
    None,
    Static(Value),
    FromStep(String, Option<String>),
    FromContext(String),
    Merge(Vec<InputSource>),
}

#[derive(Debug, Clone)]
pub enum Condition {
    Always,
    Never,
    StateEq { field: String, value: Value },
    StateGt { field: String, value: f64 },
    OutputOk { step_id: String },
    OutputFailed { step_id: String },
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone)]
pub struct Step {
    pub id: String,
    pub capability: String,
    pub max_retries: u32,
    pub input: InputSource,
    pub config_override: Option<Value>,
    pub condition: Option<Condition>,
    pub on_skip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentGlobalConfig {
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub backoff_ms: Option<u64>,
    pub base_url: Option<String>,
    pub headers: Option<Value>,
    pub environment: Option<Value>,
}

impl Default for AgentGlobalConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Some(5_000),
            max_retries: Some(3),
            backoff_ms: Some(1_000),
            base_url: None,
            headers: None,
            environment: None,
        }
    }
}

impl AgentGlobalConfig {
    pub fn merge_with_step(
        &self,
        registry_config: &CapabilityConfig,
        step_override: &Option<Value>,
    ) -> CapabilityConfig {
        let mut config = registry_config.clone();
        let _agent_environment = &self.environment;

        if let Some(timeout_ms) = self.timeout_ms {
            config.timeout_ms = timeout_ms;
        }
        if let Some(max_retries) = self.max_retries {
            config.retry_policy.max_attempts = max_retries;
        }
        if let Some(backoff_ms) = self.backoff_ms {
            config.retry_policy.backoff_ms = backoff_ms;
        }
        if self.base_url.is_some() {
            config.base_url = self.base_url.clone();
        }
        if self.headers.is_some() {
            config.headers = self.headers.clone();
        }

        Self::apply_step_override(&mut config, step_override);
        config
    }

    fn apply_step_override(config: &mut CapabilityConfig, override_value: &Option<Value>) {
        if let Some(override_value) = override_value {
            if let Some(timeout_ms) = override_value
                .get("timeout_ms")
                .and_then(|value| value.as_u64())
            {
                config.timeout_ms = timeout_ms;
            }
            if let Some(max_attempts) = override_value
                .get("max_attempts")
                .and_then(|value| value.as_u64())
                .and_then(|value| u32::try_from(value).ok())
            {
                config.retry_policy.max_attempts = max_attempts;
            }
            if let Some(backoff_ms) = override_value
                .get("backoff_ms")
                .and_then(|value| value.as_u64())
            {
                config.retry_policy.backoff_ms = backoff_ms;
            }
            if let Some(base_url) = override_value
                .get("base_url")
                .and_then(|value| value.as_str())
            {
                config.base_url = Some(base_url.to_string());
            }
            if let Some(headers) = override_value.get("headers") {
                config.headers = Some(headers.clone());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentPackage {
    pub id: String,
    pub steps: Vec<Step>,
    pub max_steps: u32,
    pub risk_config: RiskConfig,
    pub global_config: AgentGlobalConfig,
}

pub struct ExecutionLoop<'a> {
    package: AgentPackage,
    registry: &'a CapabilityRegistry,
    risk_gate: RiskGate,
}

#[derive(Debug, Clone)]
pub struct WorkerContext {
    pub state: AgentStateSnapshot,
    pub global_config: AgentGlobalConfig,
    pub risk_config: RiskConfig,
}

impl<'a> ExecutionLoop<'a> {
    pub fn new(package: AgentPackage, registry: &'a CapabilityRegistry) -> Self {
        let risk_gate = RiskGate::new(package.risk_config.clone());

        Self {
            package,
            registry,
            risk_gate,
        }
    }

    pub async fn run(&self, state: &mut State) {
        if state.paused {
            state.resume();
            if state.step_index > 0 {
                let previous_step = &self.package.steps[state.step_index - 1];
                let should_rewind = state
                    .results
                    .get(&previous_step.id)
                    .map(|result| !result.success)
                    .unwrap_or(true);

                if should_rewind {
                    state.step_index -= 1;
                    println!("[LOOP] rewound to retry failed step");
                }
            }
        }

        println!(
            "\n[LOOP] start agent={} step={}/{}",
            self.package.id,
            state.step_index,
            self.package.steps.len()
        );

        while !state.completed
            && !state.paused
            && state.step_index < self.package.steps.len()
            && (state.step_index as u32) < self.package.max_steps
        {
            let step = &self.package.steps[state.step_index];
            state.next_step(&step.id);

            println!("\n[STEP] {} (capability={})", step.id, step.capability);

            match self.should_execute_step(step, state) {
                Ok(true) => {
                    println!("  [COND] condition met");
                }
                Ok(false) => {
                    println!("  [COND] SKIPPED: condition not met");
                    if let Some(message) = &step.on_skip {
                        println!("  [COND] {}", message);
                    }
                    state.record_result(
                        &step.id,
                        StepResult {
                            success: true,
                            data: Some("\"SKIPPED\"".to_string()),
                            error: step.on_skip.clone(),
                            timestamp: Utc::now().to_rfc3339(),
                        },
                    );
                    continue;
                }
                Err(err) => {
                    println!("  [COND] ERROR evaluating condition: {}", err);
                    state.paused = true;
                    if let Err(e) = state.save() {
                        println!("  [ERROR] failed to save: {}", e);
                    }
                    break;
                }
            }

            let resolved_input = match self.resolve_input(&step.input, state, &state.results) {
                Ok(input) => {
                    if let Some(input) = &input {
                        println!("  [INPUT] resolved: {}", input);
                    }
                    input
                }
                Err(err) => {
                    println!("  [INPUT] ERROR: {}", err);
                    state.paused = true;
                    if let Err(e) = state.save() {
                        println!("  [ERROR] failed to save: {}", e);
                    }
                    break;
                }
            };

            match self.risk_gate.evaluate(&step.capability, state) {
                RiskDecision::Allow => {
                    println!("  [RISK] ALLOW");
                }
                RiskDecision::Block { reason } => {
                    println!("  [RISK] BLOCKED: {}", reason);
                    println!("  [RISK] saving snapshot & pausing");
                    state.paused = true;
                    if let Err(e) = state.save() {
                        println!("  [ERROR] failed to save: {}", e);
                    }
                    break;
                }
            }

            let (adapter, config) = match self.registry.resolve(&step.capability) {
                Some(resolved) => resolved,
                None => {
                    println!("  [ERROR] capability_not_found: {}", step.capability);
                    state.paused = true;
                    if let Err(e) = state.save() {
                        println!("  [ERROR] failed to save: {}", e);
                    }
                    break;
                }
            };

            if let Err(err) = self
                .registry
                .validate_input(&step.capability, &resolved_input)
            {
                println!("  [ERROR] input_validation_failed: {}", err);
                state.paused = true;
                if let Err(e) = state.save() {
                    println!("  [ERROR] failed to save: {}", e);
                }
                break;
            }

            let config = self
                .package
                .global_config
                .merge_with_step(config, &step.config_override);
            let mut attempt = 1;
            let max_attempts = step.max_retries.min(config.retry_policy.max_attempts);

            while attempt <= max_attempts {
                println!(
                    "  [EXEC] attempt {}/{} adapter={}",
                    attempt, max_attempts, step.capability
                );
                state.record_execution();

                match adapter.execute(resolved_input.clone(), &config).await {
                    ExecutionResult::Success(data) => {
                        let result = StepResult {
                            success: true,
                            data: Some(data.to_string()),
                            error: None,
                            timestamp: Utc::now().to_rfc3339(),
                        };
                        println!("  [OK] data={}", data);
                        state.success();
                        state.record_result(&step.id, result);
                        break;
                    }
                    ExecutionResult::Failure(err) => {
                        println!("  [FAIL] err={}", err);
                        self.record_failed_attempt(state, &step.id, attempt, max_attempts, err);
                    }
                    ExecutionResult::Timeout => {
                        println!("  [TIMEOUT]");
                        self.record_failed_attempt(
                            state,
                            &step.id,
                            attempt,
                            max_attempts,
                            "timeout".to_string(),
                        );
                    }
                }

                if !state.paused && attempt < max_attempts && config.retry_policy.backoff_ms > 0 {
                    sleep(Duration::from_millis(
                        config.retry_policy.backoff_ms.min(10),
                    ))
                    .await;
                }

                attempt += 1;
            }

            if state.paused {
                break;
            }
        }

        if state.step_index >= self.package.steps.len() && !state.paused {
            state.completed = true;
            if let Err(e) = state.save() {
                println!("  [ERROR] failed to save final state: {}", e);
            }
            println!("\n[LOOP] COMPLETED");
        } else if state.paused {
            println!("\n[LOOP] PAUSED (snapshot saved, can resume)");
        }

        self.print_summary(state);
    }

    fn should_execute_step(&self, step: &Step, state: &State) -> Result<bool, String> {
        match &step.condition {
            Some(condition) => self.evaluate_condition(condition, state),
            None => Ok(true),
        }
    }

    fn evaluate_condition(&self, condition: &Condition, state: &State) -> Result<bool, String> {
        match condition {
            Condition::Always => Ok(true),
            Condition::Never => Ok(false),
            Condition::StateEq { field, value } => {
                let current = self.get_state_field(state, field)?;
                Ok(current == *value)
            }
            Condition::StateGt { field, value } => {
                let current = self.get_state_field_numeric(state, field)?;
                Ok(current > *value)
            }
            Condition::OutputOk { step_id } => Ok(state
                .results
                .get(step_id)
                .map(|result| result.success)
                .unwrap_or(false)),
            Condition::OutputFailed { step_id } => Ok(state
                .results
                .get(step_id)
                .map(|result| !result.success)
                .unwrap_or(false)),
            Condition::And(left, right) => {
                Ok(self.evaluate_condition(left, state)?
                    && self.evaluate_condition(right, state)?)
            }
            Condition::Or(left, right) => {
                Ok(self.evaluate_condition(left, state)?
                    || self.evaluate_condition(right, state)?)
            }
            Condition::Not(condition) => Ok(!self.evaluate_condition(condition, state)?),
        }
    }

    fn get_state_field(&self, state: &State, field: &str) -> Result<Value, String> {
        match field {
            "current_step" => Ok(Value::String(state.current_step.clone())),
            "step_index" => Ok(serde_json::json!(state.step_index)),
            "consecutive_failures" => Ok(serde_json::json!(state.consecutive_failures)),
            "total_executions" => Ok(serde_json::json!(state.total_executions)),
            "completed" => Ok(Value::Bool(state.completed)),
            "paused" => Ok(Value::Bool(state.paused)),
            "agent_id" => Ok(Value::String(state.agent_id.clone())),
            "version" => Ok(Value::String(state.version.clone())),
            _ => Err(format!("unknown state field: {}", field)),
        }
    }

    fn get_state_field_numeric(&self, state: &State, field: &str) -> Result<f64, String> {
        let value = self.get_state_field(state, field)?;
        value
            .as_f64()
            .or_else(|| value.as_u64().map(|value| value as f64))
            .or_else(|| value.as_i64().map(|value| value as f64))
            .ok_or_else(|| format!("field {} is not numeric", field))
    }

    fn resolve_input(
        &self,
        source: &InputSource,
        _state: &State,
        previous_results: &HashMap<String, StepResult>,
    ) -> Result<Option<Value>, String> {
        match source {
            InputSource::None => Ok(None),
            InputSource::Static(Value::Null) => Ok(None),
            InputSource::Static(value) => Ok(Some(value.clone())),
            InputSource::FromStep(step_id, field) => {
                let result = previous_results
                    .get(step_id)
                    .ok_or_else(|| format!("step {} not found in results", step_id))?;
                if !result.success {
                    return Err(format!("step {} failed, cannot use output", step_id));
                }

                let data = result
                    .data
                    .as_ref()
                    .ok_or_else(|| format!("step {} has no data", step_id))?;
                let parsed: Value = serde_json::from_str(data)
                    .map_err(|err| format!("invalid json in step {}: {}", step_id, err))?;

                match field {
                    Some(field) => {
                        let extracted = parsed.get(field).ok_or_else(|| {
                            format!("field {} not found in step {} output", field, step_id)
                        })?;
                        Ok(Some(extracted.clone()))
                    }
                    None => Ok(Some(parsed)),
                }
            }
            InputSource::FromContext(path) => self.resolve_context(path),
            InputSource::Merge(sources) => {
                let mut merged = serde_json::Map::new();

                for source in sources {
                    if let Some(value) = self.resolve_input(source, _state, previous_results)? {
                        if let Some(object) = value.as_object() {
                            for (key, value) in object {
                                merged.insert(key.clone(), value.clone());
                            }
                        } else {
                            return Err(format!("cannot merge non-object input: {}", value));
                        }
                    }
                }

                Ok(Some(Value::Object(merged)))
            }
        }
    }

    fn resolve_context(&self, path: &str) -> Result<Option<Value>, String> {
        let parts = path.split('.').collect::<Vec<_>>();
        if parts.len() < 2 || parts[0] != "agent" || parts[1] != "environment" {
            return Err(format!("unknown context path: {}", path));
        }

        let mut current = self
            .package
            .global_config
            .environment
            .as_ref()
            .ok_or_else(|| "agent environment is not configured".to_string())?;

        for part in parts.iter().skip(2) {
            current = current
                .get(part)
                .ok_or_else(|| format!("context field {} not found in {}", part, path))?;
        }

        Ok(Some(current.clone()))
    }

    fn record_failed_attempt(
        &self,
        state: &mut State,
        step_id: &str,
        attempt: u32,
        max_retries: u32,
        err: String,
    ) {
        state.fail();

        if attempt == max_retries {
            let result = StepResult {
                success: false,
                data: None,
                error: Some(err),
                timestamp: Utc::now().to_rfc3339(),
            };
            state.record_result(step_id, result);

            state.paused = true;
            println!("  [STRATEGY] max retries exceeded -> save snapshot & pause");
            if let Err(e) = state.save() {
                println!("  [ERROR] failed to save: {}", e);
            }
        }
    }

    fn print_summary(&self, state: &State) {
        println!("\n=== SUMMARY ===");
        println!("agent: {}", self.package.id);
        println!(
            "steps executed: {}/{}",
            state.step_index,
            self.package.steps.len()
        );
        println!("total executions: {}", state.total_executions);
        println!("consecutive failures: {}", state.consecutive_failures);
        println!(
            "status: {}",
            if state.completed {
                "COMPLETED"
            } else if state.paused {
                "PAUSED"
            } else {
                "RUNNING"
            }
        );

        for (step_id, result) in &state.results {
            let status = if result.success { "ok" } else { "fail" };
            let detail = if result.success {
                result.data.as_deref().unwrap_or("no data")
            } else {
                result.error.as_deref().unwrap_or("unknown")
            };
            println!("  {} {}: {}", status, step_id, detail);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::mock::register_mocks;

    fn happy_path_package() -> AgentPackage {
        AgentPackage {
            id: "hello_test".to_string(),
            steps: vec![
                Step {
                    id: "ping".to_string(),
                    capability: "http_ping".to_string(),
                    max_retries: 2,
                    input: InputSource::None,
                    config_override: None,
                    condition: None,
                    on_skip: None,
                },
                Step {
                    id: "validate".to_string(),
                    capability: "http_validate".to_string(),
                    max_retries: 2,
                    input: InputSource::None,
                    config_override: None,
                    condition: None,
                    on_skip: None,
                },
            ],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig {
                timeout_ms: Some(5_000),
                max_retries: Some(3),
                backoff_ms: Some(1_000),
                base_url: None,
                headers: None,
                environment: None,
            },
        }
    }

    fn registry(resume_mode: bool) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        register_mocks(&mut registry, resume_mode);
        registry
    }

    #[tokio::test]
    async fn executes_multiple_steps_and_keeps_state_consistent() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        let mut state = State::new("hello_test_happy");

        loop_engine.run(&mut state).await;

        assert!(state.completed);
        assert!(!state.paused);
        assert_eq!(state.step_index, 2);
        assert_eq!(state.total_executions, 2);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.results.len(), 2);
        assert!(state.results["ping"].success);
        assert!(state.results["validate"].success);

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn pauses_and_saves_snapshot_when_retries_are_exceeded() {
        let package = happy_path_package();
        let registry = registry(false);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("pause_test");

        loop_engine.run(&mut state).await;

        assert!(!state.completed);
        assert!(state.paused);
        assert_eq!(state.step_index, 2);
        assert_eq!(state.total_executions, 3);
        assert_eq!(state.consecutive_failures, 2);
        assert!(std::path::Path::new(&state.snapshot_path()).exists());
        assert!(!state.results["validate"].success);

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn resumes_from_snapshot_and_retries_failed_step() {
        let mut state = State::new("resume_test");
        state.current_step = "validate".to_string();
        state.step_index = 2;
        state.consecutive_failures = 2;
        state.total_executions = 3;
        state.paused = true;
        state.record_result(
            "ping",
            StepResult {
                success: true,
                data: Some(serde_json::json!({"status": "ok"}).to_string()),
                error: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        );
        state.record_result(
            "validate",
            StepResult {
                success: false,
                data: None,
                error: Some("validation_failed".to_string()),
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        loop_engine.run(&mut state).await;

        assert!(state.completed);
        assert!(!state.paused);
        assert_eq!(state.step_index, 2);
        assert_eq!(state.total_executions, 4);
        assert_eq!(state.consecutive_failures, 0);
        assert!(state.results["ping"].success);
        assert!(state.results["validate"].success);

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn risk_block_saves_snapshot_and_pauses_before_execution() {
        let mut package = happy_path_package();
        package
            .risk_config
            .blacklisted_capabilities
            .insert("http_validate".to_string());
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("risk_block_test");

        loop_engine.run(&mut state).await;

        assert!(!state.completed);
        assert!(state.paused);
        assert_eq!(state.step_index, 2);
        assert_eq!(state.total_executions, 1);
        assert!(state.results["ping"].success);
        assert!(!state.results.contains_key("validate"));
        assert!(std::path::Path::new(&state.snapshot_path()).exists());

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn missing_capability_saves_snapshot_and_pauses() {
        let package = AgentPackage {
            id: "missing_test".to_string(),
            steps: vec![Step {
                id: "missing".to_string(),
                capability: "missing_capability".to_string(),
                max_retries: 2,
                input: InputSource::None,
                config_override: None,
                condition: None,
                on_skip: None,
            }],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig::default(),
        };
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("missing_capability_test");

        loop_engine.run(&mut state).await;

        assert!(state.paused);
        assert!(!state.completed);
        assert_eq!(state.total_executions, 0);
        assert!(state.results.is_empty());
        assert!(std::path::Path::new(&state.snapshot_path()).exists());

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn input_validation_pauses_before_execution() {
        let package = AgentPackage {
            id: "input_validation_test".to_string(),
            steps: vec![Step {
                id: "fetch".to_string(),
                capability: "http_get".to_string(),
                max_retries: 2,
                input: InputSource::None,
                config_override: None,
                condition: None,
                on_skip: None,
            }],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig::default(),
        };
        let mut registry = registry(true);
        crate::executor::mock::register_mock_http_get(&mut registry);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("input_validation_test");

        loop_engine.run(&mut state).await;

        assert!(state.paused);
        assert_eq!(state.total_executions, 0);
        assert!(state.results.is_empty());

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[tokio::test]
    async fn config_override_applies_to_step_execution() {
        let package = AgentPackage {
            id: "config_override_test".to_string(),
            steps: vec![Step {
                id: "ping".to_string(),
                capability: "http_ping".to_string(),
                max_retries: 2,
                input: InputSource::None,
                config_override: Some(serde_json::json!({
                    "timeout_ms": 0,
                    "max_attempts": 1
                })),
                condition: None,
                on_skip: None,
            }],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig::default(),
        };
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("config_override_test");

        loop_engine.run(&mut state).await;

        assert!(state.paused);
        assert_eq!(state.total_executions, 1);
        assert_eq!(state.results["ping"].error.as_deref(), Some("timeout"));

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[test]
    fn global_config_merges_registry_then_global_then_step() {
        let registry_config = CapabilityConfig {
            timeout_ms: 9_000,
            retry_policy: crate::capability::RetryPolicy {
                max_attempts: 9,
                backoff_ms: 9_000,
            },
            headers: Some(serde_json::json!({"X-Registry": "yes"})),
            base_url: Some("https://registry.test".to_string()),
        };
        let global = AgentGlobalConfig {
            timeout_ms: Some(3_000),
            max_retries: Some(2),
            backoff_ms: Some(500),
            base_url: Some("https://agent.test".to_string()),
            headers: Some(serde_json::json!({"User-Agent": "operant-mvp/0.7"})),
            environment: Some(serde_json::json!({"env": "test"})),
        };
        let step_override = Some(serde_json::json!({
            "timeout_ms": 15_000,
            "headers": {"X-Step": "yes"}
        }));

        let merged = global.merge_with_step(&registry_config, &step_override);

        assert_eq!(merged.timeout_ms, 15_000);
        assert_eq!(merged.retry_policy.max_attempts, 2);
        assert_eq!(merged.retry_policy.backoff_ms, 500);
        assert_eq!(merged.base_url.as_deref(), Some("https://agent.test"));
        assert_eq!(merged.headers, Some(serde_json::json!({"X-Step": "yes"})));
        assert!(global.environment.is_some());
    }

    #[test]
    fn resolves_input_from_step_field() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        let mut results = HashMap::new();
        results.insert(
            "fetch".to_string(),
            StepResult {
                success: true,
                data: Some(serde_json::json!({"url": "https://example.test/get"}).to_string()),
                error: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        let input = loop_engine
            .resolve_input(
                &InputSource::FromStep("fetch".to_string(), Some("url".to_string())),
                &State::new("routing_test"),
                &results,
            )
            .unwrap();

        assert_eq!(
            input,
            Some(Value::String("https://example.test/get".to_string()))
        );
    }

    #[test]
    fn resolves_merged_input_sources() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        let mut results = HashMap::new();
        results.insert(
            "fetch".to_string(),
            StepResult {
                success: true,
                data: Some(serde_json::json!({"headers": {"Host": "httpbin.org"}}).to_string()),
                error: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        let input = loop_engine
            .resolve_input(
                &InputSource::Merge(vec![
                    InputSource::Static(serde_json::json!({"path": "/get"})),
                    InputSource::FromStep("fetch".to_string(), Some("headers".to_string())),
                ]),
                &State::new("routing_test"),
                &results,
            )
            .unwrap();

        assert_eq!(
            input,
            Some(serde_json::json!({"path": "/get", "Host": "httpbin.org"}))
        );
    }

    #[test]
    fn resolves_context_environment_path() {
        let mut package = happy_path_package();
        package.global_config.environment = Some(serde_json::json!({
            "api_key": "test_key_123"
        }));
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(package, &registry);

        let input = loop_engine
            .resolve_input(
                &InputSource::FromContext("agent.environment.api_key".to_string()),
                &State::new("routing_test"),
                &HashMap::new(),
            )
            .unwrap();

        assert_eq!(input, Some(Value::String("test_key_123".to_string())));
    }

    #[test]
    fn errors_when_step_reference_is_missing() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);

        let err = loop_engine
            .resolve_input(
                &InputSource::FromStep("missing".to_string(), None),
                &State::new("routing_test"),
                &HashMap::new(),
            )
            .unwrap_err();

        assert!(err.contains("not found"));
    }

    #[test]
    fn evaluates_state_and_output_conditions() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        let mut state = State::new("condition_test");
        state.total_executions = 2;
        state.results.insert(
            "fetch".to_string(),
            StepResult {
                success: true,
                data: Some(serde_json::json!({"ok": true}).to_string()),
                error: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        assert!(loop_engine
            .evaluate_condition(
                &Condition::And(
                    Box::new(Condition::OutputOk {
                        step_id: "fetch".to_string()
                    }),
                    Box::new(Condition::StateGt {
                        field: "total_executions".to_string(),
                        value: 1.0
                    })
                ),
                &state
            )
            .unwrap());
        assert!(loop_engine
            .evaluate_condition(
                &Condition::StateEq {
                    field: "agent_id".to_string(),
                    value: Value::String("condition_test".to_string())
                },
                &state
            )
            .unwrap());
    }

    #[test]
    fn evaluates_composite_conditions() {
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(happy_path_package(), &registry);
        let state = State::new("condition_test");

        assert!(!loop_engine
            .evaluate_condition(&Condition::Never, &state)
            .unwrap());
        assert!(loop_engine
            .evaluate_condition(
                &Condition::Or(
                    Box::new(Condition::Never),
                    Box::new(Condition::Not(Box::new(Condition::Never)))
                ),
                &state
            )
            .unwrap());
    }

    #[tokio::test]
    async fn skipped_step_is_recorded_without_execution() {
        let package = AgentPackage {
            id: "skip_test".to_string(),
            steps: vec![Step {
                id: "skip_me".to_string(),
                capability: "http_ping".to_string(),
                max_retries: 2,
                input: InputSource::None,
                config_override: None,
                condition: Some(Condition::Never),
                on_skip: Some("not needed".to_string()),
            }],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig::default(),
        };
        let registry = registry(true);
        let loop_engine = ExecutionLoop::new(package, &registry);
        let mut state = State::new("skip_test");

        loop_engine.run(&mut state).await;

        assert!(state.completed);
        assert_eq!(state.total_executions, 0);
        assert_eq!(
            state.results["skip_me"].data.as_deref(),
            Some("\"SKIPPED\"")
        );

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[test]
    fn worker_context_keeps_package_config_separate_from_state_snapshot() {
        let mut package = happy_path_package();
        package.global_config.base_url = Some("https://example.test".to_string());
        let mut state = State::new("worker_context_test");
        state.total_executions = 7;

        let context = WorkerContext {
            state: state.snapshot(),
            global_config: package.global_config.clone(),
            risk_config: package.risk_config.clone(),
        };

        assert_eq!(context.state.agent_id, "worker_context_test");
        assert_eq!(context.state.total_executions, 7);
        assert!(context.global_config.base_url.is_some());
        assert_eq!(context.risk_config.max_total_executions, 100);
    }
}
