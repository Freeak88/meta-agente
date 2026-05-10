#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_example(json: &str) -> SimulationReport {
        serde_json::from_str(json).expect("JSON debe parsear")
    }

    fn roundtrip(report: &SimulationReport) -> SimulationReport {
        let json = serde_json::to_string(report).expect("debe serializar");
        serde_json::from_str(&json).expect("debe deserializar")
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
}
