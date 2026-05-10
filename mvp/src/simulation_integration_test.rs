use crate::r#loop::{AgentGlobalConfig, AgentPackage, InputSource, Step};
use crate::risk::RiskConfig;
use crate::simulation::{
    SimulationConfig, SimulationEngine, SimulationMode, SimulationStatus, ValidationStatus,
};

fn build_mvp_package() -> AgentPackage {
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
                id: "fetch".to_string(),
                capability: "http_get".to_string(),
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
        global_config: AgentGlobalConfig::default(),
    }
}

#[tokio::test]
async fn test_mvp_package_adversarial_rejected() {
    let package = build_mvp_package();
    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Adversarial,
        seed: None,
        max_runs: Some(1),
        failure_rate: None,
        execution_log_id: None,
    };

    let report = SimulationEngine::run(&package, &config).await;

    assert_eq!(report.agent_id, "hello_test");
    assert_eq!(report.mode, SimulationMode::Adversarial);
    assert_eq!(report.summary.validation_status, ValidationStatus::Rejected);
    assert_eq!(report.summary.pass_rate, 0.0);

    let validate_failure = report
        .summary
        .predicted_failures
        .iter()
        .find(|failure| failure.step_id == "validate");
    assert!(
        validate_failure.is_some(),
        "validate debe estar en predicted_failures"
    );
    assert_eq!(validate_failure.unwrap().probability, 1.0);

    assert_eq!(report.runs.len(), 1);
    let run = &report.runs[0];
    assert_eq!(run.status, SimulationStatus::Fail);
    assert!(!run.results.get("ping").unwrap().success);
    assert!(!run.results.get("fetch").unwrap().success);
    assert!(!run.results.get("validate").unwrap().success);

    assert!(!report.summary.recommendations.is_empty());
    assert!(report
        .summary
        .recommendations
        .iter()
        .any(|recommendation| recommendation.contains("fails")));
}

#[tokio::test]
async fn test_mvp_package_optimistic_approved() {
    let package = build_mvp_package();
    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Optimistic,
        seed: None,
        max_runs: Some(1),
        failure_rate: None,
        execution_log_id: None,
    };

    let report = SimulationEngine::run(&package, &config).await;

    assert_eq!(report.summary.validation_status, ValidationStatus::Approved);
    assert_eq!(report.summary.pass_rate, 1.0);
    assert!(report.summary.predicted_failures.is_empty());

    let run = &report.runs[0];
    assert_eq!(run.status, SimulationStatus::Pass);
    assert!(run.results.get("ping").unwrap().success);
    assert!(run.results.get("fetch").unwrap().success);
    assert!(run.results.get("validate").unwrap().success);
}

#[tokio::test]
async fn test_mvp_package_stochastic_needs_review() {
    let package = build_mvp_package();
    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Stochastic,
        seed: Some(42),
        max_runs: Some(100),
        failure_rate: Some(0.25),
        execution_log_id: None,
    };

    let report = SimulationEngine::run(&package, &config).await;

    assert!(
        report.summary.pass_rate > 0.6 && report.summary.pass_rate < 0.9,
        "pass_rate debe estar entre 60% y 90%, fue {}",
        report.summary.pass_rate
    );
    assert_ne!(report.summary.validation_status, ValidationStatus::Approved);
    if report.summary.pass_rate < 1.0 {
        assert!(!report.summary.predicted_failures.is_empty());
    }
    assert_eq!(report.runs.len(), 100);
}

#[tokio::test]
async fn test_mvp_package_deterministic_seed() {
    let package = build_mvp_package();
    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Stochastic,
        seed: Some(12_345),
        max_runs: Some(10),
        failure_rate: Some(0.5),
        execution_log_id: None,
    };

    let report1 = SimulationEngine::run(&package, &config).await;
    let report2 = SimulationEngine::run(&package, &config).await;

    assert_eq!(report1.summary.pass_rate, report2.summary.pass_rate);
    assert_eq!(report1.runs.len(), report2.runs.len());

    for (run1, run2) in report1.runs.iter().zip(report2.runs.iter()) {
        assert_eq!(run1.status, run2.status);
        for (step_id, result1) in &run1.results {
            let result2 = run2.results.get(step_id).unwrap();
            assert_eq!(result1.success, result2.success);
        }
    }
}
