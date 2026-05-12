#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRecord {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub worker_id: String,
    pub duration_ms: u64,
    pub steps_total: u32,
    pub steps_success: u32,
    pub steps_failed: u32,
    pub status: ExecutionStatus,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Completed,
    Failed,
    Paused,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailureRecord {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub worker_id: String,
    pub step_id: String,
    pub error_type: String,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentHealth {
    pub agent_id: String,
    pub version: String,
    pub total_executions: u64,
    pub success_rate_24h: f64,
    pub avg_duration_ms: u64,
    pub last_execution: Option<DateTime<Utc>>,
    pub status: AgentStatus,
    pub active_workers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountUsage {
    pub account_id: String,
    pub period: String,
    pub total_executions: u32,
    pub total_cost: f64,
    pub total_storage_mb: u64,
    pub agents_deployed: u32,
    pub plan_limit_reached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetrics {
    pub total_agents: u32,
    pub total_workers: u32,
    pub total_executions_24h: u64,
    pub system_load_percent: f64,
    pub avg_response_time_ms: u64,
}

pub struct MetricsCollector {
    executions: Vec<ExecutionRecord>,
    failures: Vec<FailureRecord>,
    agent_health: HashMap<String, AgentHealth>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            executions: Vec::new(),
            failures: Vec::new(),
            agent_health: HashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_execution(
        &mut self,
        agent_id: &str,
        worker_id: &str,
        duration_ms: u64,
        steps_total: u32,
        steps_success: u32,
        steps_failed: u32,
        status: ExecutionStatus,
        cost: f64,
    ) {
        self.executions.push(ExecutionRecord {
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            worker_id: worker_id.to_string(),
            duration_ms,
            steps_total,
            steps_success,
            steps_failed,
            status,
            cost,
        });

        self.recompute_agent_health(agent_id);
    }

    pub fn record_failure(
        &mut self,
        agent_id: &str,
        worker_id: &str,
        step_id: &str,
        error_type: &str,
        retry_count: u32,
    ) {
        self.failures.push(FailureRecord {
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            worker_id: worker_id.to_string(),
            step_id: step_id.to_string(),
            error_type: error_type.to_string(),
            retry_count,
        });
    }

    pub fn agent_health(&self, agent_id: &str) -> Option<&AgentHealth> {
        self.agent_health.get(agent_id)
    }

    pub fn account_usage(&self, account_id: &str, period: &str) -> AccountUsage {
        let executions: Vec<&ExecutionRecord> = self
            .executions
            .iter()
            .filter(|execution| {
                execution.agent_id.starts_with(account_id)
                    && execution.timestamp.format("%Y-%m").to_string() == period
            })
            .collect();
        let total_cost = executions.iter().map(|execution| execution.cost).sum();

        AccountUsage {
            account_id: account_id.to_string(),
            period: period.to_string(),
            total_executions: executions.len() as u32,
            total_cost,
            total_storage_mb: 0,
            agents_deployed: self.agent_health.len() as u32,
            plan_limit_reached: false,
        }
    }

    pub fn system_metrics(&self) -> SystemMetrics {
        let day_ago = Utc::now() - Duration::days(1);
        let recent_executions: Vec<&ExecutionRecord> = self
            .executions
            .iter()
            .filter(|execution| execution.timestamp > day_ago)
            .collect();
        let total_duration: u64 = recent_executions
            .iter()
            .map(|execution| execution.duration_ms)
            .sum();
        let avg_response_time_ms = if recent_executions.is_empty() {
            0
        } else {
            total_duration / recent_executions.len() as u64
        };

        SystemMetrics {
            total_agents: self.agent_health.len() as u32,
            total_workers: self
                .agent_health
                .values()
                .map(|health| health.active_workers)
                .sum(),
            total_executions_24h: recent_executions.len() as u64,
            system_load_percent: (recent_executions.len() as f64 / 10_000.0).min(1.0) * 100.0,
            avg_response_time_ms,
        }
    }

    pub fn top_failures(&self, limit: usize) -> Vec<(String, u32)> {
        let mut counts: HashMap<String, u32> = HashMap::new();
        for failure in &self.failures {
            *counts.entry(failure.error_type.clone()).or_insert(0) += 1;
        }

        let mut sorted: Vec<(String, u32)> = counts.into_iter().collect();
        sorted.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        sorted.truncate(limit);
        sorted
    }

    fn recompute_agent_health(&mut self, agent_id: &str) {
        let agent_executions: Vec<&ExecutionRecord> = self
            .executions
            .iter()
            .filter(|execution| execution.agent_id == agent_id)
            .collect();
        let recent_cutoff = Utc::now() - Duration::days(1);
        let recent: Vec<&ExecutionRecord> = agent_executions
            .iter()
            .copied()
            .filter(|execution| execution.timestamp > recent_cutoff)
            .collect();
        let total_executions = agent_executions.len() as u64;
        let success_count = recent
            .iter()
            .filter(|execution| execution.status == ExecutionStatus::Completed)
            .count();
        let success_rate_24h = if recent.is_empty() {
            1.0
        } else {
            success_count as f64 / recent.len() as f64
        };
        let avg_duration_ms = if agent_executions.is_empty() {
            0
        } else {
            agent_executions
                .iter()
                .map(|execution| execution.duration_ms)
                .sum::<u64>()
                / agent_executions.len() as u64
        };
        let last_execution = agent_executions
            .iter()
            .map(|execution| execution.timestamp)
            .max();
        let active_workers = agent_executions
            .iter()
            .map(|execution| execution.worker_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let status = if success_rate_24h < 0.5 {
            AgentStatus::Unhealthy
        } else if success_rate_24h < 0.8 {
            AgentStatus::Degraded
        } else {
            AgentStatus::Healthy
        };

        self.agent_health.insert(
            agent_id.to_string(),
            AgentHealth {
                agent_id: agent_id.to_string(),
                version: "unknown".to_string(),
                total_executions,
                success_rate_24h,
                avg_duration_ms,
                last_execution,
                status,
                active_workers,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_execution_and_updates_health() {
        let mut metrics = MetricsCollector::new();

        metrics.record_execution(
            "agent_1",
            "worker_1",
            5_000,
            5,
            5,
            0,
            ExecutionStatus::Completed,
            0.05,
        );

        let health = metrics.agent_health("agent_1").unwrap();
        assert_eq!(health.total_executions, 1);
        assert_eq!(health.status, AgentStatus::Healthy);
        assert_eq!(health.success_rate_24h, 1.0);
    }

    #[test]
    fn failed_executions_degrade_health() {
        let mut metrics = MetricsCollector::new();

        for status in [
            ExecutionStatus::Completed,
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
        ] {
            let steps_success = if status == ExecutionStatus::Completed {
                5
            } else {
                0
            };
            let steps_failed = if status == ExecutionStatus::Failed {
                5
            } else {
                0
            };
            metrics.record_execution(
                "agent_1",
                "worker_1",
                1_000,
                5,
                steps_success,
                steps_failed,
                status,
                0.05,
            );
        }

        let health = metrics.agent_health("agent_1").unwrap();
        assert!(health.success_rate_24h < 0.8);
        assert_eq!(health.status, AgentStatus::Degraded);
    }

    #[test]
    fn repeated_failures_make_agent_unhealthy() {
        let mut metrics = MetricsCollector::new();

        for _ in 0..5 {
            metrics.record_execution(
                "agent_1",
                "worker_1",
                1_000,
                5,
                0,
                5,
                ExecutionStatus::Failed,
                0.0,
            );
        }

        let health = metrics.agent_health("agent_1").unwrap();
        assert_eq!(health.success_rate_24h, 0.0);
        assert_eq!(health.status, AgentStatus::Unhealthy);
    }

    #[test]
    fn account_usage_sums_period_costs() {
        let mut metrics = MetricsCollector::new();

        metrics.record_execution(
            "acct_1_agent_a",
            "w1",
            1_000,
            3,
            3,
            0,
            ExecutionStatus::Completed,
            0.03,
        );
        metrics.record_execution(
            "acct_1_agent_b",
            "w2",
            2_000,
            5,
            5,
            0,
            ExecutionStatus::Completed,
            0.05,
        );

        let period = Utc::now().format("%Y-%m").to_string();
        let usage = metrics.account_usage("acct_1", &period);

        assert_eq!(usage.total_executions, 2);
        assert!(usage.total_cost > 0.07);
    }

    #[test]
    fn system_metrics_summarize_recent_activity() {
        let mut metrics = MetricsCollector::new();

        for index in 0..10 {
            metrics.record_execution(
                &format!("agent_{}", index % 3),
                &format!("worker_{}", index),
                1_000,
                3,
                3,
                0,
                ExecutionStatus::Completed,
                0.03,
            );
        }

        let system = metrics.system_metrics();

        assert_eq!(system.total_agents, 3);
        assert_eq!(system.total_executions_24h, 10);
        assert_eq!(system.avg_response_time_ms, 1_000);
    }

    #[test]
    fn top_failures_are_counted_and_sorted() {
        let mut metrics = MetricsCollector::new();

        metrics.record_failure("a1", "w1", "s1", "timeout", 2);
        metrics.record_failure("a1", "w1", "s2", "timeout", 1);
        metrics.record_failure("a1", "w1", "s3", "validation", 0);
        metrics.record_failure("a2", "w2", "s1", "timeout", 3);

        let top = metrics.top_failures(2);

        assert_eq!(top[0], ("timeout".to_string(), 3));
        assert_eq!(top[1], ("validation".to_string(), 1));
    }
}
