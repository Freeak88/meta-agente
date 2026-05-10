#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::r#loop::{AgentGlobalConfig, AgentPackage, InputSource, Step};
use crate::risk::RiskConfig;

pub use crate::dsl_parser::parse_opl;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OplAst {
    pub agent: AgentDecl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentDecl {
    pub id: String,
    pub mission: String,
    pub properties: HashMap<String, OplValue>,
    pub steps: Vec<StepDecl>,
    pub fallbacks: Vec<FallbackDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepDecl {
    pub id: String,
    pub capability: String,
    pub properties: HashMap<String, OplValue>,
    pub risk: Option<String>,
    pub hitl: Option<bool>,
    pub input: Option<OplValue>,
    pub validate: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FallbackDecl {
    pub condition: String,
    pub target: String,
    pub retry: Option<u32>,
    pub backoff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum OplValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<OplValue>),
    Object(HashMap<String, OplValue>),
    Identifier(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranspileError {
    EmptyAgent,
}

pub struct OplTranspiler;

impl OplTranspiler {
    pub fn transpile(ast: &OplAst) -> Result<AgentPackage, TranspileError> {
        if ast.agent.id.trim().is_empty() {
            return Err(TranspileError::EmptyAgent);
        }

        Ok(AgentPackage {
            id: ast.agent.id.clone(),
            steps: ast.agent.steps.iter().map(Self::transpile_step).collect(),
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
        })
    }

    fn transpile_step(step: &StepDecl) -> Step {
        Step {
            id: step.id.clone(),
            capability: step.capability.clone(),
            max_retries: 2,
            input: match &step.input {
                Some(value) => InputSource::Static(Self::value_to_json(value)),
                _ => InputSource::None,
            },
            config_override: None,
            condition: None,
            on_skip: None,
        }
    }

    fn value_to_json(value: &OplValue) -> serde_json::Value {
        match value {
            OplValue::String(value) | OplValue::Identifier(value) => {
                serde_json::Value::String(value.clone())
            }
            OplValue::Number(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            OplValue::Bool(value) => serde_json::Value::Bool(*value),
            OplValue::List(values) => {
                serde_json::Value::Array(values.iter().map(Self::value_to_json).collect())
            }
            OplValue::Object(values) => serde_json::Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::value_to_json(value)))
                    .collect(),
            ),
        }
    }
}
