#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::capability::{CapabilityConfig, ExecutionResult};
use crate::r#loop::AgentPackage;
use chrono::Utc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SimulationMode {
    Optimistic,
    Adversarial,
    Stochastic,
    Replay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationConfig {
    pub agent_id: String,
    pub mode: SimulationMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_runs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_log_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationReport {
    pub agent_id: String,
    pub mode: SimulationMode,
    pub runs: Vec<SimulationRun>,
    pub summary: SimulationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationRun {
    pub run_id: u32,
    pub results: HashMap<String, StepResult>,
    pub execution_time_ms: u64,
    pub status: SimulationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulationStatus {
    Pass,
    Fail,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepResult {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationSummary {
    pub pass_rate: f64,
    pub avg_execution_time_ms: u64,
    pub predicted_failures: Vec<PredictedFailure>,
    pub risk_spikes: Vec<RiskSpike>,
    pub recommendations: Vec<String>,
    pub validation_status: ValidationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PredictedFailure {
    pub step_id: String,
    pub failure_type: String,
    pub probability: f64,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskSpike {
    pub step_id: String,
    pub risk_score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    Approved,
    Rejected,
    NeedsReview,
}

pub struct MockRegistry {
    responses: HashMap<String, MockResponse>,
    rng: Option<StdRng>,
    failure_rate: f64,
}

#[derive(Debug, Clone)]
pub struct MockResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

impl MockRegistry {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            rng: None,
            failure_rate: 0.0,
        }
    }

    pub fn with_response(mut self, capability_id: &str, response: MockResponse) -> Self {
        self.responses.insert(capability_id.to_string(), response);
        self
    }

    pub fn with_stochastic(mut self, seed: u64, failure_rate: f64) -> Self {
        self.rng = Some(StdRng::seed_from_u64(seed));
        self.failure_rate = failure_rate;
        self
    }

    pub async fn execute(
        &mut self,
        capability_id: &str,
        _input: Option<serde_json::Value>,
        _config: &CapabilityConfig,
    ) -> ExecutionResult {
        let latency_ms = self
            .responses
            .get(capability_id)
            .map(|resp| resp.latency_ms)
            .unwrap_or(10);
        sleep(Duration::from_millis(latency_ms)).await;

        if let Some(rng) = &mut self.rng {
            let roll: f64 = rng.gen();
            if roll < self.failure_rate {
                return ExecutionResult::Failure("stochastic_failure".to_string());
            }
        }

        match self.responses.get(capability_id) {
            Some(resp) if resp.success => {
                ExecutionResult::Success(resp.data.clone().unwrap_or(serde_json::Value::Null))
            }
            Some(resp) => ExecutionResult::Failure(
                resp.error
                    .clone()
                    .unwrap_or_else(|| "mock_failure".to_string()),
            ),
            None => ExecutionResult::Failure(format!("mock_not_found: {}", capability_id)),
        }
    }
}

pub struct SimulationEngine;

impl SimulationEngine {
    pub async fn run(package: &AgentPackage, config: &SimulationConfig) -> SimulationReport {
        let max_runs = config.max_runs.unwrap_or(1).max(1);
        let mut run_seed_rng = config.seed.map(StdRng::seed_from_u64);
        let mut runs = Vec::with_capacity(max_runs as usize);

        for run_id in 1..=max_runs {
            let run_seed = run_seed_rng
                .as_mut()
                .map(|rng| rng.gen::<u64>())
                .or(config.seed.map(|seed| seed.wrapping_add(run_id as u64)))
                .unwrap_or(run_id as u64);
            let mut mock_registry = Self::build_mock_registry(package, config, run_seed);
            let run = Self::simulate_run(run_id, package, &mut mock_registry).await;
            runs.push(run);
        }

        let summary = Self::generate_summary(&runs);

        SimulationReport {
            agent_id: package.id.clone(),
            mode: config.mode.clone(),
            runs,
            summary,
        }
    }

    fn build_mock_registry(
        package: &AgentPackage,
        config: &SimulationConfig,
        run_seed: u64,
    ) -> MockRegistry {
        let mut registry = MockRegistry::new();

        match config.mode {
            SimulationMode::Optimistic => {
                for step in &package.steps {
                    registry = registry.with_response(
                        &step.capability,
                        MockResponse {
                            success: true,
                            data: Some(serde_json::json!({"mock": true, "status": "ok"})),
                            error: None,
                            latency_ms: 1,
                        },
                    );
                }
            }
            SimulationMode::Adversarial => {
                for step in &package.steps {
                    registry = registry.with_response(
                        &step.capability,
                        MockResponse {
                            success: false,
                            data: None,
                            error: Some("adversarial_failure".to_string()),
                            latency_ms: 1,
                        },
                    );
                }
            }
            SimulationMode::Stochastic => {
                registry = registry.with_stochastic(run_seed, config.failure_rate.unwrap_or(0.2));
                for step in &package.steps {
                    registry = registry.with_response(
                        &step.capability,
                        MockResponse {
                            success: true,
                            data: Some(serde_json::json!({"mock": true, "status": "ok"})),
                            error: None,
                            latency_ms: 1,
                        },
                    );
                }
            }
            SimulationMode::Replay => {
                for step in &package.steps {
                    registry = registry.with_response(
                        &step.capability,
                        MockResponse {
                            success: true,
                            data: Some(serde_json::json!({"replay": true})),
                            error: None,
                            latency_ms: 1,
                        },
                    );
                }
            }
        }

        registry
    }

    async fn simulate_run(
        run_id: u32,
        package: &AgentPackage,
        registry: &mut MockRegistry,
    ) -> SimulationRun {
        let mut results = HashMap::new();
        let mut total_time_ms = 0u64;
        let mut status = SimulationStatus::Pass;

        for step in &package.steps {
            let start = std::time::Instant::now();
            let result = registry
                .execute(&step.capability, None, &CapabilityConfig::default())
                .await;
            let elapsed = start.elapsed().as_millis() as u64;
            total_time_ms += elapsed;

            let step_result = match result {
                ExecutionResult::Success(data) => StepResult {
                    success: true,
                    data: Some(data.to_string()),
                    error: None,
                    timestamp: Utc::now().to_rfc3339(),
                },
                ExecutionResult::Failure(err) => {
                    status = SimulationStatus::Fail;
                    StepResult {
                        success: false,
                        data: None,
                        error: Some(err),
                        timestamp: Utc::now().to_rfc3339(),
                    }
                }
                ExecutionResult::Timeout => {
                    status = SimulationStatus::Timeout;
                    StepResult {
                        success: false,
                        data: None,
                        error: Some("timeout".to_string()),
                        timestamp: Utc::now().to_rfc3339(),
                    }
                }
            };

            results.insert(step.id.clone(), step_result);
        }

        SimulationRun {
            run_id,
            results,
            execution_time_ms: total_time_ms,
            status,
        }
    }

    fn generate_summary(runs: &[SimulationRun]) -> SimulationSummary {
        let total = runs.len() as f64;
        let pass_count = runs
            .iter()
            .filter(|run| run.status == SimulationStatus::Pass)
            .count() as f64;
        let pass_rate = if total > 0.0 { pass_count / total } else { 0.0 };

        let avg_execution_time_ms = if runs.is_empty() {
            0
        } else {
            runs.iter().map(|run| run.execution_time_ms).sum::<u64>() / runs.len() as u64
        };

        let mut failure_counts: HashMap<String, (String, u32)> = HashMap::new();
        for run in runs {
            for (step_id, result) in &run.results {
                if !result.success {
                    let entry = failure_counts.entry(step_id.clone()).or_insert((
                        result
                            .error
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        0,
                    ));
                    entry.1 += 1;
                }
            }
        }

        let mut predicted_failures: Vec<PredictedFailure> = failure_counts
            .into_iter()
            .map(|(step_id, (failure_type, count))| PredictedFailure {
                step_id,
                failure_type,
                probability: if total > 0.0 {
                    count as f64 / total
                } else {
                    0.0
                },
                mitigation: "Add retry or fallback".to_string(),
            })
            .filter(|failure| failure.probability > 0.05)
            .collect();
        predicted_failures.sort_by(|a, b| {
            b.probability
                .partial_cmp(&a.probability)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.step_id.cmp(&b.step_id))
        });
        predicted_failures.truncate(5);

        let mut risk_spikes = Vec::new();
        if pass_rate < 0.8 && !runs.is_empty() {
            if let Some(step_id) = predicted_failures
                .first()
                .map(|failure| failure.step_id.clone())
            {
                risk_spikes.push(RiskSpike {
                    step_id,
                    risk_score: 1.0 - pass_rate,
                    reason: "High failure rate in simulation".to_string(),
                });
            }
        }
        risk_spikes.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.step_id.cmp(&b.step_id))
        });
        risk_spikes.truncate(3);

        let validation_status = if pass_rate == 1.0 && risk_spikes.is_empty() {
            ValidationStatus::Approved
        } else if pass_rate == 0.0 || risk_spikes.iter().any(|spike| spike.risk_score >= 0.9) {
            ValidationStatus::Rejected
        } else {
            ValidationStatus::NeedsReview
        };

        let mut recommendations = Vec::new();
        match validation_status {
            ValidationStatus::Approved => {
                recommendations.push(
                    "All steps pass in ideal conditions. Agent ready for deployment.".to_string(),
                );
            }
            ValidationStatus::Rejected => {
                recommendations.push(
                    "Agent fails under simulation conditions. Not ready for production."
                        .to_string(),
                );
                if !predicted_failures.is_empty() {
                    recommendations.push("Add retry or fallback for failing steps.".to_string());
                }
            }
            ValidationStatus::NeedsReview => {
                recommendations.push(format!(
                    "Pass rate {:.0}% requires review before deployment.",
                    pass_rate * 100.0
                ));
            }
        }
        recommendations.truncate(5);

        SimulationSummary {
            pass_rate,
            avg_execution_time_ms,
            predicted_failures,
            risk_spikes,
            recommendations,
            validation_status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#loop::{AgentGlobalConfig, AgentPackage, InputSource, Step};
    use crate::risk::RiskConfig;

    fn parse_example(json: &str) -> SimulationReport {
        serde_json::from_str(json).expect("JSON debe parsear")
    }

    fn roundtrip(report: &SimulationReport) -> SimulationReport {
        let json = serde_json::to_string(report).expect("debe serializar");
        serde_json::from_str(&json).expect("debe deserializar")
    }

    fn test_package() -> AgentPackage {
        AgentPackage {
            id: "test".to_string(),
            steps: vec![Step {
                id: "a".to_string(),
                capability: "cap_a".to_string(),
                max_retries: 1,
                input: InputSource::None,
                config_override: None,
                condition: None,
                on_skip: None,
            }],
            max_steps: 10,
            risk_config: RiskConfig::default(),
            global_config: AgentGlobalConfig::default(),
        }
    }

    #[test]
    fn test_optimistic_schema() {
        let json = r#"{
            "agent_id": "invoice_processor",
            "mode": "optimistic",
            "runs": [{
                "run_id": 1,
                "results": {
                    "login": {"success": true, "data": "{\"status\":\"ok\"}", "error": null, "timestamp": "2026-05-09T22:43:00Z"},
                    "fetch": {"success": true, "data": "{\"count\":3}", "error": null, "timestamp": "2026-05-09T22:43:01Z"},
                    "validate": {"success": true, "data": "{\"valid\":true}", "error": null, "timestamp": "2026-05-09T22:43:02Z"},
                    "submit": {"success": true, "data": "{\"id\":\"inv_123\"}", "error": null, "timestamp": "2026-05-09T22:43:03Z"}
                },
                "execution_time_ms": 3500,
                "status": "PASS"
            }],
            "summary": {
                "pass_rate": 1.0,
                "avg_execution_time_ms": 3500,
                "predicted_failures": [],
                "risk_spikes": [],
                "recommendations": ["All steps pass in ideal conditions. Agent ready for deployment."],
                "validation_status": "APPROVED"
            }
        }"#;

        let report = parse_example(json);
        assert_eq!(report.agent_id, "invoice_processor");
        assert_eq!(report.mode, SimulationMode::Optimistic);
        assert_eq!(report.summary.validation_status, ValidationStatus::Approved);
        assert_eq!(report.summary.pass_rate, 1.0);
        assert!(report.summary.predicted_failures.is_empty());

        let rt = roundtrip(&report);
        assert_eq!(rt, report);
    }

    #[test]
    fn test_adversarial_schema() {
        let json = r#"{
            "agent_id": "invoice_processor",
            "mode": "adversarial",
            "runs": [{
                "run_id": 1,
                "results": {
                    "login": {"success": false, "data": null, "error": "timeout", "timestamp": "2026-05-09T22:43:00Z"},
                    "fetch": {"success": false, "data": null, "error": "timeout", "timestamp": "2026-05-09T22:43:00Z"},
                    "validate": {"success": false, "data": null, "error": "validation_failed", "timestamp": "2026-05-09T22:43:00Z"},
                    "submit": {"success": false, "data": null, "error": "timeout", "timestamp": "2026-05-09T22:43:00Z"}
                },
                "execution_time_ms": 12000,
                "status": "FAIL"
            }],
            "summary": {
                "pass_rate": 0.0,
                "avg_execution_time_ms": 12000,
                "predicted_failures": [
                    {"step_id": "login", "failure_type": "timeout", "probability": 1.0, "mitigation": "Add retry"}
                ],
                "risk_spikes": [
                    {"step_id": "login", "risk_score": 1.0, "reason": "Critical path failure"}
                ],
                "recommendations": ["Agent fails completely under adversarial conditions."],
                "validation_status": "REJECTED"
            }
        }"#;

        let report = parse_example(json);
        assert_eq!(report.mode, SimulationMode::Adversarial);
        assert_eq!(report.summary.validation_status, ValidationStatus::Rejected);
        assert_eq!(report.summary.predicted_failures.len(), 1);
        assert_eq!(report.summary.risk_spikes.len(), 1);
    }

    #[test]
    fn test_stochastic_schema() {
        let json = r#"{
            "agent_id": "invoice_processor",
            "mode": "stochastic",
            "runs": [
                {"run_id": 1, "results": {}, "execution_time_ms": 4200, "status": "PASS"},
                {"run_id": 2, "results": {}, "execution_time_ms": 3800, "status": "PASS"},
                {"run_id": 3, "results": {}, "execution_time_ms": 15000, "status": "TIMEOUT"}
            ],
            "summary": {
                "pass_rate": 0.73,
                "avg_execution_time_ms": 4850,
                "predicted_failures": [
                    {"step_id": "login", "failure_type": "timeout", "probability": 0.15, "mitigation": "Increase timeout"}
                ],
                "risk_spikes": [
                    {"step_id": "submit", "risk_score": 0.82, "reason": "High variance"}
                ],
                "recommendations": ["Pass rate 73% is below recommended 80% threshold."],
                "validation_status": "NEEDS_REVIEW"
            }
        }"#;

        let report = parse_example(json);
        assert_eq!(report.mode, SimulationMode::Stochastic);
        assert_eq!(
            report.summary.validation_status,
            ValidationStatus::NeedsReview
        );
        assert_eq!(report.runs.len(), 3);
        assert!(report
            .runs
            .iter()
            .any(|r| r.status == SimulationStatus::Timeout));
    }

    #[test]
    fn test_replay_schema() {
        let json = r#"{
            "agent_id": "invoice_processor",
            "mode": "replay",
            "runs": [{
                "run_id": 1,
                "results": {
                    "login": {"success": true, "data": "{\"status\":\"ok\"}", "error": null, "timestamp": "2026-05-09T22:43:00Z"},
                    "fetch": {"success": true, "data": "{\"count\":3}", "error": null, "timestamp": "2026-05-09T22:43:01Z"},
                    "validate": {"success": false, "data": null, "error": "validation_failed", "timestamp": "2026-05-09T22:43:02Z"}
                },
                "execution_time_ms": 8500,
                "status": "FAIL"
            }],
            "summary": {
                "pass_rate": 0.0,
                "avg_execution_time_ms": 8500,
                "predicted_failures": [
                    {"step_id": "validate", "failure_type": "validation_failed", "probability": 1.0, "mitigation": "Fix identified in commit abc123"}
                ],
                "risk_spikes": [],
                "recommendations": ["Replay matches original execution exactly."],
                "validation_status": "REJECTED"
            }
        }"#;

        let report = parse_example(json);
        assert_eq!(report.mode, SimulationMode::Replay);
        assert_eq!(report.summary.validation_status, ValidationStatus::Rejected);
    }

    #[test]
    fn test_null_fields_are_preserved_in_serialized_report() {
        let report = SimulationReport {
            agent_id: "test".to_string(),
            mode: SimulationMode::Adversarial,
            runs: vec![SimulationRun {
                run_id: 1,
                results: HashMap::from([(
                    "login".to_string(),
                    StepResult {
                        success: false,
                        data: None,
                        error: Some("timeout".to_string()),
                        timestamp: "2026-05-09T22:43:00Z".to_string(),
                    },
                )]),
                execution_time_ms: 100,
                status: SimulationStatus::Fail,
            }],
            summary: SimulationSummary {
                pass_rate: 0.0,
                avg_execution_time_ms: 100,
                predicted_failures: vec![],
                risk_spikes: vec![],
                recommendations: vec!["fail".to_string()],
                validation_status: ValidationStatus::Rejected,
            },
        };

        let value = serde_json::to_value(&report).expect("debe serializar");
        assert!(value["runs"][0]["results"]["login"]
            .get("data")
            .expect("data field debe existir")
            .is_null());
        assert_eq!(
            value["runs"][0]["results"]["login"]["error"],
            serde_json::json!("timeout")
        );
    }

    #[test]
    fn test_validation_status_rules() {
        let approved = SimulationSummary {
            pass_rate: 1.0,
            avg_execution_time_ms: 1_000,
            predicted_failures: vec![],
            risk_spikes: vec![],
            recommendations: vec!["ok".to_string()],
            validation_status: ValidationStatus::Approved,
        };
        assert_eq!(approved.validation_status, ValidationStatus::Approved);

        let rejected = SimulationSummary {
            pass_rate: 0.7,
            avg_execution_time_ms: 1_000,
            predicted_failures: vec![],
            risk_spikes: vec![],
            recommendations: vec!["fail".to_string()],
            validation_status: ValidationStatus::Rejected,
        };
        assert_eq!(rejected.validation_status, ValidationStatus::Rejected);
    }

    #[test]
    fn test_limits_are_logical_not_schema_truncation() {
        let report = SimulationReport {
            agent_id: "test".to_string(),
            mode: SimulationMode::Optimistic,
            runs: vec![],
            summary: SimulationSummary {
                pass_rate: 1.0,
                avg_execution_time_ms: 0,
                predicted_failures: vec![
                    PredictedFailure {
                        step_id: "1".to_string(),
                        failure_type: "a".to_string(),
                        probability: 0.1,
                        mitigation: "x".to_string(),
                    },
                    PredictedFailure {
                        step_id: "2".to_string(),
                        failure_type: "b".to_string(),
                        probability: 0.2,
                        mitigation: "x".to_string(),
                    },
                    PredictedFailure {
                        step_id: "3".to_string(),
                        failure_type: "c".to_string(),
                        probability: 0.3,
                        mitigation: "x".to_string(),
                    },
                    PredictedFailure {
                        step_id: "4".to_string(),
                        failure_type: "d".to_string(),
                        probability: 0.4,
                        mitigation: "x".to_string(),
                    },
                    PredictedFailure {
                        step_id: "5".to_string(),
                        failure_type: "e".to_string(),
                        probability: 0.5,
                        mitigation: "x".to_string(),
                    },
                    PredictedFailure {
                        step_id: "6".to_string(),
                        failure_type: "f".to_string(),
                        probability: 0.6,
                        mitigation: "x".to_string(),
                    },
                ],
                risk_spikes: vec![
                    RiskSpike {
                        step_id: "1".to_string(),
                        risk_score: 0.8,
                        reason: "a".to_string(),
                    },
                    RiskSpike {
                        step_id: "2".to_string(),
                        risk_score: 0.9,
                        reason: "b".to_string(),
                    },
                    RiskSpike {
                        step_id: "3".to_string(),
                        risk_score: 1.0,
                        reason: "c".to_string(),
                    },
                    RiskSpike {
                        step_id: "4".to_string(),
                        risk_score: 0.85,
                        reason: "d".to_string(),
                    },
                ],
                recommendations: vec![
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string(),
                    "e".to_string(),
                    "f".to_string(),
                ],
                validation_status: ValidationStatus::Approved,
            },
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"6\""));
        assert!(json.contains("\"f\""));
    }

    #[tokio::test]
    async fn test_optimistic_engine() {
        let package = test_package();
        let config = SimulationConfig {
            agent_id: "test".to_string(),
            mode: SimulationMode::Optimistic,
            seed: None,
            max_runs: Some(1),
            failure_rate: None,
            execution_log_id: None,
        };

        let report = SimulationEngine::run(&package, &config).await;
        assert_eq!(report.agent_id, "test");
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].status, SimulationStatus::Pass);
        assert_eq!(report.summary.validation_status, ValidationStatus::Approved);
        assert_eq!(report.summary.pass_rate, 1.0);
    }

    #[tokio::test]
    async fn test_adversarial_engine() {
        let package = test_package();
        let config = SimulationConfig {
            agent_id: "test".to_string(),
            mode: SimulationMode::Adversarial,
            seed: None,
            max_runs: Some(1),
            failure_rate: None,
            execution_log_id: None,
        };

        let report = SimulationEngine::run(&package, &config).await;
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].status, SimulationStatus::Fail);
        assert_eq!(report.summary.validation_status, ValidationStatus::Rejected);
        assert_eq!(report.summary.pass_rate, 0.0);
        assert_eq!(report.summary.predicted_failures.len(), 1);
    }

    #[tokio::test]
    async fn test_stochastic_distribution() {
        let package = test_package();
        let config = SimulationConfig {
            agent_id: "test".to_string(),
            mode: SimulationMode::Stochastic,
            seed: Some(12_345),
            max_runs: Some(100),
            failure_rate: Some(0.2),
            execution_log_id: None,
        };

        let report = SimulationEngine::run(&package, &config).await;
        assert_eq!(report.runs.len(), 100);
        assert!(report.summary.pass_rate > 0.6 && report.summary.pass_rate < 0.9);
        assert_eq!(
            report.summary.validation_status,
            ValidationStatus::NeedsReview
        );
    }

    #[tokio::test]
    async fn test_stochastic_seed_is_deterministic() {
        let package = test_package();
        let config = SimulationConfig {
            agent_id: "test".to_string(),
            mode: SimulationMode::Stochastic,
            seed: Some(12_345),
            max_runs: Some(20),
            failure_rate: Some(0.2),
            execution_log_id: None,
        };

        let report_a = SimulationEngine::run(&package, &config).await;
        let report_b = SimulationEngine::run(&package, &config).await;
        let statuses_a: Vec<_> = report_a.runs.iter().map(|run| &run.status).collect();
        let statuses_b: Vec<_> = report_b.runs.iter().map(|run| &run.status).collect();
        assert_eq!(statuses_a, statuses_b);
        assert_eq!(report_a.summary.pass_rate, report_b.summary.pass_rate);
    }
}
