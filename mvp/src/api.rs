#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::meta_agent::HumanIntent;
use crate::simulation::SimulationReport;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAgentRequest {
    pub intent: HumanIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAgentResponse {
    pub agent_id: String,
    pub version: String,
    pub hash: String,
    pub opl: String,
    pub simulation_report: SimulationReport,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub errors: Vec<String>,
}
