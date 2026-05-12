use std::collections::HashMap;

use crate::dsl::{
    AgentDecl, FallbackDecl, OplAst, OplTranspiler, OplValue, StepDecl, TranspileError,
};
use crate::r#loop::{AgentGlobalConfig, AgentPackage, InputSource, Step};
use crate::risk::RiskConfig;

fn build_minimal_ast() -> OplAst {
    let mut agent_props = HashMap::new();
    agent_props.insert(
        "domain".to_string(),
        OplValue::String("testing".to_string()),
    );
    agent_props.insert("autonomy".to_string(), OplValue::String("full".to_string()));

    OplAst {
        agent: AgentDecl {
            id: "hello_test".to_string(),
            mission: "Validar MVP end-to-end".to_string(),
            properties: agent_props,
            steps: vec![
                StepDecl {
                    id: "ping".to_string(),
                    capability: "http_ping".to_string(),
                    properties: HashMap::new(),
                    risk: Some("LOW".to_string()),
                    hitl: None,
                    input: None,
                    validate: vec![],
                },
                StepDecl {
                    id: "fetch".to_string(),
                    capability: "http_get".to_string(),
                    properties: HashMap::new(),
                    risk: None,
                    hitl: None,
                    input: Some(OplValue::String("/get".to_string())),
                    validate: vec![],
                },
                StepDecl {
                    id: "validate".to_string(),
                    capability: "http_validate".to_string(),
                    properties: HashMap::new(),
                    risk: Some("HIGH".to_string()),
                    hitl: Some(true),
                    input: None,
                    validate: vec![],
                },
            ],
            fallbacks: vec![FallbackDecl {
                condition: "timeout".to_string(),
                target: "ping".to_string(),
                retry: Some(2),
                backoff: Some("EXPONENTIAL".to_string()),
            }],
        },
        imports: vec![],
    }
}

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
                input: InputSource::Static(serde_json::Value::String("/get".to_string())),
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
            max_retries: Some(2),
            backoff_ms: Some(1_000),
            base_url: None,
            headers: None,
            environment: None,
        },
    }
}

fn package_contract_json(package: &AgentPackage, ast: &OplAst) -> serde_json::Value {
    let steps: Vec<_> = package
        .steps
        .iter()
        .map(|step| {
            let decl = ast
                .agent
                .steps
                .iter()
                .find(|decl| decl.id == step.id)
                .expect("step declaration exists");
            let input = match &step.input {
                InputSource::None => serde_json::Value::Null,
                InputSource::Static(value) => serde_json::json!({ "Static": value }),
                _ => serde_json::Value::Null,
            };

            let mut item = serde_json::json!({
                "id": step.id,
                "capability": step.capability,
                "max_retries": step.max_retries,
                "input": input,
                "config_override": null,
                "condition": null,
                "on_skip": null
            });

            if let Some(risk) = &decl.risk {
                item["risk"] = serde_json::json!(risk);
            }
            if let Some(hitl) = decl.hitl {
                item["hitl"] = serde_json::json!(hitl);
            }

            item
        })
        .collect();

    let blacklisted: Vec<String> = package
        .risk_config
        .blacklisted_capabilities
        .iter()
        .cloned()
        .collect();

    serde_json::json!({
        "id": package.id,
        "mission": ast.agent.mission,
        "steps": steps,
        "max_steps": package.max_steps,
        "risk_config": {
            "blacklisted_capabilities": blacklisted,
            "max_consecutive_failures": package.risk_config.max_consecutive_failures,
            "max_total_executions": package.risk_config.max_total_executions
        },
        "global_config": {
            "timeout_ms": package.global_config.timeout_ms,
            "max_retries": package.global_config.max_retries,
            "backoff_ms": package.global_config.backoff_ms,
            "base_url": package.global_config.base_url,
            "headers": package.global_config.headers,
            "environment": package.global_config.environment
        },
        "fallbacks": ast.agent.fallbacks
    })
}

#[test]
fn test_ast_serializes_to_json() {
    let ast = build_minimal_ast();
    let json = serde_json::to_string_pretty(&ast).expect("debe serializar");

    assert!(json.contains("\"id\": \"hello_test\""));
    assert!(json.contains("\"mission\": \"Validar MVP end-to-end\""));
    assert!(json.contains("\"capability\": \"http_ping\""));
    assert!(json.contains("\"risk\": \"LOW\""));
    assert!(json.contains("\"risk\": \"HIGH\""));
    assert!(json.contains("\"hitl\": true"));
    assert!(json.contains("\"condition\": \"timeout\""));
    assert!(json.contains("\"backoff\": \"EXPONENTIAL\""));
}

#[test]
fn test_ast_roundtrip() {
    let ast = build_minimal_ast();
    let json = serde_json::to_string(&ast).expect("debe serializar");
    let ast2: OplAst = serde_json::from_str(&json).expect("debe deserializar");
    assert_eq!(ast, ast2);
}

#[test]
fn test_transpile_ast_to_package() {
    let ast = build_minimal_ast();
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");
    let mvp = build_mvp_package();

    assert_eq!(package.id, mvp.id);
    assert_eq!(package.max_steps, mvp.max_steps);
    assert_eq!(package.steps.len(), mvp.steps.len());

    for (actual, expected) in package.steps.iter().zip(mvp.steps.iter()) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.capability, expected.capability);
        assert_eq!(actual.max_retries, expected.max_retries);
    }

    assert!(matches!(package.steps[0].input, InputSource::None));
    assert!(matches!(package.steps[1].input, InputSource::Static(_)));
    assert!(matches!(package.steps[2].input, InputSource::None));
}

#[test]
fn test_package_json_matches_contract() {
    let ast = build_minimal_ast();
    let mvp = build_mvp_package();
    let json = package_contract_json(&mvp, &ast);

    let expected = serde_json::json!({
        "id": "hello_test",
        "mission": "Validar MVP end-to-end",
        "steps": [
            {
                "id": "ping",
                "capability": "http_ping",
                "max_retries": 2,
                "input": null,
                "config_override": null,
                "condition": null,
                "on_skip": null,
                "risk": "LOW"
            },
            {
                "id": "fetch",
                "capability": "http_get",
                "max_retries": 2,
                "input": { "Static": "/get" },
                "config_override": null,
                "condition": null,
                "on_skip": null
            },
            {
                "id": "validate",
                "capability": "http_validate",
                "max_retries": 2,
                "input": null,
                "config_override": null,
                "condition": null,
                "on_skip": null,
                "risk": "HIGH",
                "hitl": true
            }
        ],
        "max_steps": 10,
        "risk_config": {
            "blacklisted_capabilities": [],
            "max_consecutive_failures": 5,
            "max_total_executions": 100
        },
        "global_config": {
            "timeout_ms": 5000,
            "max_retries": 2,
            "backoff_ms": 1000,
            "base_url": null,
            "headers": null,
            "environment": null
        },
        "fallbacks": [
            {
                "condition": "timeout",
                "target": "ping",
                "retry": 2,
                "backoff": "EXPONENTIAL"
            }
        ]
    });

    assert_eq!(json, expected);
}

#[test]
fn test_opl_to_json_equivalence() {
    let ast = build_minimal_ast();
    let package = OplTranspiler::transpile(&ast).expect("debe transpilar");
    let json = package_contract_json(&package, &ast);

    assert_eq!(json["id"], "hello_test");
    assert_eq!(json["steps"].as_array().unwrap().len(), 3);
    assert_eq!(json["steps"][0]["capability"], "http_ping");
    assert_eq!(
        json["steps"][1]["input"],
        serde_json::json!({ "Static": "/get" })
    );
    assert_eq!(json["steps"][2]["risk"], "HIGH");
}

#[test]
fn test_transpile_rejects_empty_agent_id() {
    let mut ast = build_minimal_ast();
    ast.agent.id = " ".to_string();

    let error = OplTranspiler::transpile(&ast).expect_err("debe rechazar agente vacio");
    assert_eq!(error, TranspileError::EmptyAgent);
}
