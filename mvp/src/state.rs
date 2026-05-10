use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub current_step: String,
    pub step_index: usize,
    pub consecutive_failures: u32,
    pub total_executions: u32,
    pub completed: bool,
    pub paused: bool,
    pub results: HashMap<String, StepResult>,
    pub agent_id: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct AgentStateSnapshot {
    pub current_step: String,
    pub step_index: usize,
    pub consecutive_failures: u32,
    pub total_executions: u32,
    pub completed: bool,
    pub paused: bool,
    pub previous_results: HashMap<String, StepResult>,
    pub agent_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl State {
    pub fn new(agent_id: &str) -> Self {
        Self {
            current_step: String::new(),
            step_index: 0,
            consecutive_failures: 0,
            total_executions: 0,
            completed: false,
            paused: false,
            results: HashMap::new(),
            agent_id: agent_id.to_string(),
            version: "0.1".to_string(),
        }
    }

    pub fn record_result(&mut self, step_id: &str, result: StepResult) {
        self.results.insert(step_id.to_string(), result);
    }

    pub fn merge_results(&mut self, results: Vec<(String, StepResult)>) {
        for (step_id, result) in results {
            self.record_result(&step_id, result);
        }
    }

    pub fn record_execution(&mut self) {
        self.total_executions += 1;
    }

    pub fn next_step(&mut self, step_id: &str) {
        self.current_step = step_id.to_string();
        self.step_index += 1;
    }

    pub fn fail(&mut self) {
        self.consecutive_failures += 1;
    }

    pub fn success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn snapshot(&self) -> AgentStateSnapshot {
        AgentStateSnapshot {
            current_step: self.current_step.clone(),
            step_index: self.step_index,
            consecutive_failures: self.consecutive_failures,
            total_executions: self.total_executions,
            completed: self.completed,
            paused: self.paused,
            previous_results: self.results.clone(),
            agent_id: self.agent_id.clone(),
            version: self.version.clone(),
        }
    }

    pub fn snapshot_path(&self) -> String {
        format!("./snapshots/{}_{}.json", self.agent_id, self.version)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = self.snapshot_path();
        let dir = Path::new("./snapshots");
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;

        println!("[STATE] snapshot saved to {}", path);
        Ok(())
    }

    pub fn load(agent_id: &str, version: &str) -> Result<Self, String> {
        let path = format!("./snapshots/{}_{}.json", agent_id, version);
        let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let state: State = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        println!("[STATE] snapshot loaded from {}", path);
        Ok(state)
    }

    pub fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            println!(
                "[STATE] resumed from step {} ({})",
                self.step_index, self.current_step
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_snapshot() {
        let mut state = State::new("test_agent_save_load");
        state.step_index = 1;
        state.paused = true;
        state.record_result(
            "step1",
            StepResult {
                success: true,
                data: Some("ok".to_string()),
                error: None,
                timestamp: "2026-05-05T00:00:00Z".to_string(),
            },
        );

        state.save().unwrap();

        let loaded = State::load("test_agent_save_load", "0.1").unwrap();
        assert_eq!(loaded.step_index, 1);
        assert!(loaded.paused);
        assert_eq!(loaded.results.len(), 1);
        assert_eq!(loaded.agent_id, "test_agent_save_load");

        std::fs::remove_file(state.snapshot_path()).unwrap();
    }

    #[test]
    fn resume_clears_pause() {
        let mut state = State::new("test_resume");
        state.paused = true;
        state.step_index = 2;

        state.resume();

        assert!(!state.paused);
    }

    #[test]
    fn snapshot_copies_state_fields() {
        let mut state = State::new("test_snapshot");
        state.current_step = "fetch".to_string();
        state.step_index = 2;
        state.consecutive_failures = 1;
        state.total_executions = 4;
        state.paused = true;
        state.record_result(
            "fetch",
            StepResult {
                success: true,
                data: Some("{\"ok\":true}".to_string()),
                error: None,
                timestamp: "2026-05-05T00:00:00Z".to_string(),
            },
        );

        let snapshot = state.snapshot();

        assert_eq!(snapshot.agent_id, "test_snapshot");
        assert_eq!(snapshot.version, "0.1");
        assert_eq!(snapshot.current_step, "fetch");
        assert_eq!(snapshot.step_index, 2);
        assert_eq!(snapshot.consecutive_failures, 1);
        assert_eq!(snapshot.total_executions, 4);
        assert!(!snapshot.completed);
        assert!(snapshot.paused);
        assert!(snapshot.previous_results.contains_key("fetch"));
    }

    #[test]
    fn snapshot_is_independent_clone() {
        let mut state = State::new("test_snapshot_clone");
        state.record_result(
            "before",
            StepResult {
                success: true,
                data: Some("ok".to_string()),
                error: None,
                timestamp: "2026-05-05T00:00:00Z".to_string(),
            },
        );

        let snapshot = state.snapshot();
        state.record_result(
            "after",
            StepResult {
                success: true,
                data: Some("later".to_string()),
                error: None,
                timestamp: "2026-05-05T00:00:01Z".to_string(),
            },
        );

        assert!(snapshot.previous_results.contains_key("before"));
        assert!(!snapshot.previous_results.contains_key("after"));
        assert_eq!(state.results.len(), 2);
        assert_eq!(snapshot.previous_results.len(), 1);
    }

    #[test]
    fn merge_results_records_outputs_without_advancing_step_index() {
        let mut state = State::new("test_merge");
        state.step_index = 3;

        state.merge_results(vec![(
            "parallel_a".to_string(),
            StepResult {
                success: true,
                data: Some("ok".to_string()),
                error: None,
                timestamp: "2026-05-05T00:00:00Z".to_string(),
            },
        )]);

        assert!(state.results.contains_key("parallel_a"));
        assert_eq!(state.step_index, 3);
        assert_eq!(state.total_executions, 0);
    }
}
