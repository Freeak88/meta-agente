#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::r#loop::AgentPackage;
use crate::simulation::SimulationReport;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanIntent {
    pub raw_input: String,
    pub goal: String,
    pub domain: Option<String>,
    pub constraints: Vec<String>,
    pub environment: Option<String>,
    pub urgency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterpretedIntent {
    pub goal: String,
    pub domain: String,
    pub required_capabilities: Vec<String>,
    pub constraints: Vec<String>,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedOpl {
    pub source: String,
    pub intent_hash: String,
}

#[derive(Debug, Clone)]
pub struct MetaAgentResult {
    pub opl: GeneratedOpl,
    pub package: AgentPackage,
    pub simulation: SimulationReport,
    pub status: String,
}
