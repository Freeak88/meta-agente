use crate::dsl::{parse_opl, OplTranspiler, OplValue};

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

#[test]
fn test_parse_minimal_opl() {
    let ast = parse_opl(get_minimal_opl()).expect("debe parsear");

    assert_eq!(ast.agent.id, "hello_test");
    assert_eq!(ast.agent.mission, "Validar MVP end-to-end");
    assert_eq!(
        ast.agent.properties.get("domain"),
        Some(&OplValue::String("testing".to_string()))
    );
    assert_eq!(
        ast.agent.properties.get("autonomy"),
        Some(&OplValue::String("full".to_string()))
    );
}

#[test]
fn test_parse_steps() {
    let ast = parse_opl(get_minimal_opl()).expect("debe parsear");

    assert_eq!(ast.agent.steps.len(), 3);

    assert_eq!(ast.agent.steps[0].id, "ping");
    assert_eq!(ast.agent.steps[0].capability, "http_ping");
    assert_eq!(ast.agent.steps[0].risk, Some("LOW".to_string()));
    assert_eq!(ast.agent.steps[0].hitl, None);

    assert_eq!(ast.agent.steps[1].id, "fetch");
    assert_eq!(ast.agent.steps[1].capability, "http_get");
    assert_eq!(
        ast.agent.steps[1].input,
        Some(OplValue::String("/get".to_string()))
    );

    assert_eq!(ast.agent.steps[2].id, "validate");
    assert_eq!(ast.agent.steps[2].capability, "http_validate");
    assert_eq!(ast.agent.steps[2].risk, Some("HIGH".to_string()));
    assert_eq!(ast.agent.steps[2].hitl, Some(true));
}

#[test]
fn test_parse_fallbacks() {
    let ast = parse_opl(get_minimal_opl()).expect("debe parsear");

    assert_eq!(ast.agent.fallbacks.len(), 1);
    assert_eq!(ast.agent.fallbacks[0].condition, "timeout");
    assert_eq!(ast.agent.fallbacks[0].target, "ping");
    assert_eq!(ast.agent.fallbacks[0].retry, Some(2));
    assert_eq!(
        ast.agent.fallbacks[0].backoff,
        Some("EXPONENTIAL".to_string())
    );
}

#[test]
fn test_parse_error_unparsed() {
    let bad_opl = r#"
CREATE AGENT test
FOR "test"
WITH
  domain = "test"

STEP a
  USES cap_a

EXTRA STUFF THAT DOES NOT PARSE
"#;

    let result = parse_opl(bad_opl);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unparsed"));
}

#[test]
fn test_parse_error_invalid() {
    let bad_opl = "NOT AN AGENT DECLARATION";
    let result = parse_opl(bad_opl);
    assert!(result.is_err());
}

#[test]
fn test_transpile_matches_contract() {
    let ast = parse_opl(get_minimal_opl()).expect("debe parsear");
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");

    assert_eq!(package.id, "hello_test");
    assert_eq!(package.steps.len(), 3);
    assert_eq!(package.steps[0].id, "ping");
    assert_eq!(package.steps[1].id, "fetch");
    assert_eq!(package.steps[2].id, "validate");
}
