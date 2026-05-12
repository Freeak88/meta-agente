mod api;
#[cfg(test)]
mod api_contract_test;
mod api_server;
#[cfg(test)]
mod api_server_test;
mod capability;
mod dsl;
#[cfg(test)]
mod dsl_contract_test;
#[cfg(test)]
mod dsl_e2e_test;
mod dsl_parser;
#[cfg(test)]
mod dsl_parser_test;
mod executor;
mod r#loop;
mod mcp;
mod mcp_stdio;
mod meta_agent;
#[cfg(test)]
mod meta_agent_contract_test;
#[cfg(test)]
mod meta_agent_test;
mod openapi_bridge;
mod risk;
mod simulation;
#[cfg(test)]
mod simulation_integration_test;
mod skill_registry;
mod state;

use capability::{CapabilityConfig, CapabilityRegistry, RetryPolicy};
use executor::http::HttpGetAdapter;
use executor::mock::{register_mock_http_get, register_mocks};
use r#loop::{AgentGlobalConfig, AgentPackage, Condition, ExecutionLoop, InputSource, Step};
use risk::RiskConfig;
use state::State;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--mock-mcp-stdio-server".to_string()) {
        mcp_stdio::run_mock_stdio_server().await;
        return;
    }

    if args.contains(&"--server".to_string()) {
        api_server::start_server().await;
        return;
    }

    println!("=== OPERANT MVP v0.9 ===\n");

    let resume = args.contains(&"--resume".to_string());
    let block_validate = args.contains(&"--block-validate".to_string());
    let strict = args.contains(&"--strict".to_string());
    let use_mock_http = args.contains(&"--mock-http".to_string());

    let mut registry = CapabilityRegistry::new();
    register_mocks(&mut registry, resume);

    if use_mock_http {
        register_mock_http_get(&mut registry);
        println!("[REGISTRY] http_get (MOCK) registered");
    } else {
        let http_config = CapabilityConfig {
            timeout_ms: 5_000,
            retry_policy: RetryPolicy {
                max_attempts: 3,
                backoff_ms: 1_000,
            },
            headers: None,
            base_url: Some("https://httpbin.org".to_string()),
        };
        registry.register(
            Box::new(HttpGetAdapter::new(Some("https://httpbin.org".to_string()))),
            Some(http_config),
        );
        println!("[REGISTRY] http_get (REAL) registered");
    }
    println!("[REGISTRY] all capabilities: {:?}", registry.list());

    let mut risk_config = if strict {
        RiskConfig::strict()
    } else {
        RiskConfig::default()
    };
    if block_validate {
        risk_config
            .blacklisted_capabilities
            .insert("http_validate".to_string());
    }

    // Agent Package hardcodeado: esto es lo unico que define el agente.
    let package = AgentPackage {
        id: "hello_test".to_string(),
        steps: vec![
            Step {
                id: "fetch".to_string(),
                capability: "http_get".to_string(),
                max_retries: 2,
                input: InputSource::Static(serde_json::json!("/get")),
                config_override: None,
                condition: None,
                on_skip: None,
            },
            Step {
                id: "extract".to_string(),
                capability: "http_get".to_string(),
                max_retries: 2,
                input: InputSource::FromStep("fetch".to_string(), Some("url".to_string())),
                config_override: None,
                condition: Some(Condition::OutputOk {
                    step_id: "fetch".to_string(),
                }),
                on_skip: Some("fetch failed, cannot extract".to_string()),
            },
            Step {
                id: "cleanup".to_string(),
                capability: "http_get".to_string(),
                max_retries: 1,
                input: InputSource::Static(serde_json::json!("/status/200")),
                config_override: None,
                condition: Some(Condition::OutputFailed {
                    step_id: "fetch".to_string(),
                }),
                on_skip: Some("fetch ok, no cleanup needed".to_string()),
            },
            Step {
                id: "merge_test".to_string(),
                capability: "http_get".to_string(),
                max_retries: 2,
                input: InputSource::Merge(vec![
                    InputSource::Static(serde_json::json!({
                        "path": "/get",
                        "query": {"source": "merge"}
                    })),
                    InputSource::FromStep("fetch".to_string(), Some("headers".to_string())),
                ]),
                config_override: None,
                condition: Some(Condition::And(
                    Box::new(Condition::OutputOk {
                        step_id: "fetch".to_string(),
                    }),
                    Box::new(Condition::StateGt {
                        field: "total_executions".to_string(),
                        value: 1.0,
                    }),
                )),
                on_skip: Some("merge prerequisites not met".to_string()),
            },
            Step {
                id: "context_test".to_string(),
                capability: "http_get".to_string(),
                max_retries: 2,
                input: InputSource::Merge(vec![
                    InputSource::Static(serde_json::json!({"path": "/get"})),
                    InputSource::Static(serde_json::json!({"query": {"from": "context"}})),
                    InputSource::FromContext("agent.environment".to_string()),
                ]),
                config_override: Some(serde_json::json!({
                    "timeout_ms": 15_000
                })),
                condition: Some(Condition::Or(
                    Box::new(Condition::StateEq {
                        field: "agent_id".to_string(),
                        value: serde_json::json!("hello_test"),
                    }),
                    Box::new(Condition::Always),
                )),
                on_skip: Some("context disabled".to_string()),
            },
            Step {
                id: "never_runs".to_string(),
                capability: "http_get".to_string(),
                max_retries: 1,
                input: InputSource::Static(serde_json::json!("/status/204")),
                config_override: None,
                condition: Some(Condition::And(
                    Box::new(Condition::Never),
                    Box::new(Condition::Not(Box::new(Condition::Always))),
                )),
                on_skip: Some("demonstrating skip".to_string()),
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
        risk_config,
        global_config: AgentGlobalConfig {
            timeout_ms: Some(3_000),
            max_retries: Some(2),
            backoff_ms: Some(500),
            base_url: Some("https://httpbin.org".to_string()),
            headers: Some(serde_json::json!({
                "User-Agent": "operant-mvp/0.9"
            })),
            environment: Some(serde_json::json!({
                "api_key": "test_key_123"
            })),
        },
    };

    let mut state = if resume {
        match State::load("hello_test", "0.1") {
            Ok(s) => s,
            Err(e) => {
                println!("[ERROR] No snapshot found: {}. Starting fresh.", e);
                State::new("hello_test")
            }
        }
    } else {
        State::new("hello_test")
    };

    let loop_engine = ExecutionLoop::new(package, &registry);

    loop_engine.run(&mut state).await;

    if state.paused {
        println!("\n[INFO] Run with --resume to continue from snapshot");
    }
}
