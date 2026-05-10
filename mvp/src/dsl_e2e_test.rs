use crate::dsl::{parse_opl, OplTranspiler};
use crate::simulation::{SimulationConfig, SimulationEngine, SimulationMode, ValidationStatus};

fn get_minimal_opl() -> &'static str {
    r#"
CREATE AGENT hello_test
FOR "Validar MVP end-to-end"
WITH
  domain = "testing",
  autonomy = "full"

STEP ping
  USES http_ping
  RISK LOW

STEP fetch
  USES http_get
  INPUT "/get"

STEP validate
  USES http_validate
  RISK HIGH
  HITL true

ON timeout
  FALLBACK TO ping
  RETRY 2
  BACKOFF EXPONENTIAL
"#
}

#[tokio::test]
async fn test_e2e_opl_to_simulation_optimistic() {
    let ast = parse_opl(get_minimal_opl()).expect("OPL debe parsear");
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

    assert_eq!(package.id, "hello_test");
    assert_eq!(package.steps.len(), 3);
    assert_eq!(package.steps[0].id, "ping");
    assert_eq!(package.steps[1].id, "fetch");
    assert_eq!(package.steps[2].id, "validate");

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
    assert!(run.results.get("ping").unwrap().success);
    assert!(run.results.get("fetch").unwrap().success);
    assert!(run.results.get("validate").unwrap().success);
}

#[tokio::test]
async fn test_e2e_opl_to_simulation_adversarial() {
    let ast = parse_opl(get_minimal_opl()).expect("OPL debe parsear");
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Adversarial,
        seed: None,
        max_runs: Some(1),
        failure_rate: None,
        execution_log_id: None,
    };

    let report = SimulationEngine::run(&package, &config).await;

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

    let run = &report.runs[0];
    assert!(!run.results.get("ping").unwrap().success);
    assert!(!run.results.get("fetch").unwrap().success);
    assert!(!run.results.get("validate").unwrap().success);
}

#[tokio::test]
async fn test_e2e_opl_to_simulation_stochastic() {
    let ast = parse_opl(get_minimal_opl()).expect("OPL debe parsear");
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

    let config = SimulationConfig {
        agent_id: "hello_test".to_string(),
        mode: SimulationMode::Stochastic,
        seed: Some(42),
        max_runs: Some(50),
        failure_rate: Some(0.3),
        execution_log_id: None,
    };

    let report = SimulationEngine::run(&package, &config).await;

    assert!(
        report.summary.pass_rate > 0.1 && report.summary.pass_rate < 0.9,
        "pass_rate debe estar entre 10% y 90%, fue {}",
        report.summary.pass_rate
    );
    assert_ne!(report.summary.validation_status, ValidationStatus::Approved);
    assert_eq!(report.runs.len(), 50);
}

#[tokio::test]
async fn test_e2e_opl_deterministic() {
    let ast = parse_opl(get_minimal_opl()).expect("OPL debe parsear");
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

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
    }
}
