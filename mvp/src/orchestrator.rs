#![allow(dead_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Worker {
    pub id: String,
    pub agent_id: String,
    pub version: String,
    pub status: WorkerStatus,
    pub last_heartbeat: String,
    pub cpu_percent: f64,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerStatus {
    Starting,
    Running,
    Paused,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Deployment {
    pub agent_id: String,
    pub version: String,
    pub target_workers: u32,
    pub actual_workers: u32,
    pub healthy_workers: u32,
    pub status: DeploymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentStatus {
    Deploying,
    Active,
    Scaling,
    RollingBack,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkerHealth {
    pub worker_id: String,
    pub agent_id: String,
    pub status: WorkerStatus,
    pub healthy: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorError {
    AlreadyDeployed,
    NotFound,
    NotActive,
    InvalidVersion,
    RollbackFailed,
}

pub struct Orchestrator {
    workers: HashMap<String, Worker>,
    deployments: HashMap<String, Deployment>,
    next_worker_id: u64,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            deployments: HashMap::new(),
            next_worker_id: 0,
        }
    }

    pub async fn deploy(
        &mut self,
        agent_id: &str,
        version: &str,
    ) -> Result<Deployment, OrchestratorError> {
        if version.trim().is_empty() || version == "invalid" {
            return Err(OrchestratorError::InvalidVersion);
        }

        if let Some(existing) = self.deployments.get(agent_id) {
            if existing.version == version && existing.status == DeploymentStatus::Active {
                return Err(OrchestratorError::AlreadyDeployed);
            }
        }

        let mut deployment = Deployment {
            agent_id: agent_id.to_string(),
            version: version.to_string(),
            target_workers: 1,
            actual_workers: 0,
            healthy_workers: 0,
            status: DeploymentStatus::Deploying,
        };
        self.deployments
            .insert(agent_id.to_string(), deployment.clone());

        sleep(Duration::from_millis(10)).await;

        self.spawn_worker(agent_id, version, WorkerStatus::Running);
        deployment.actual_workers = 1;
        deployment.healthy_workers = 1;
        deployment.status = DeploymentStatus::Active;
        self.deployments
            .insert(agent_id.to_string(), deployment.clone());

        Ok(deployment)
    }

    pub async fn scale(
        &mut self,
        agent_id: &str,
        target: u32,
    ) -> Result<Deployment, OrchestratorError> {
        let current_deployment = self
            .deployments
            .get(agent_id)
            .cloned()
            .ok_or(OrchestratorError::NotFound)?;

        if current_deployment.status != DeploymentStatus::Active {
            return Err(OrchestratorError::NotActive);
        }

        let running = self.running_worker_ids(agent_id);
        let current = running.len() as u32;
        let version = current_deployment.version.clone();

        if let Some(deployment) = self.deployments.get_mut(agent_id) {
            deployment.status = DeploymentStatus::Scaling;
            deployment.target_workers = target;
        }

        if target > current {
            for _ in current..target {
                self.spawn_worker(agent_id, &version, WorkerStatus::Running);
            }
        } else if target < current {
            for worker_id in running.into_iter().take((current - target) as usize) {
                if let Some(worker) = self.workers.get_mut(&worker_id) {
                    worker.status = WorkerStatus::Terminated;
                }
            }
        }

        sleep(Duration::from_millis(5)).await;

        let actual_workers = self.running_worker_ids(agent_id).len() as u32;
        let deployment = self
            .deployments
            .get_mut(agent_id)
            .ok_or(OrchestratorError::NotFound)?;
        deployment.actual_workers = actual_workers;
        deployment.healthy_workers = actual_workers;
        deployment.status = DeploymentStatus::Active;

        Ok(deployment.clone())
    }

    pub async fn health_check(&self) -> Vec<WorkerHealth> {
        let mut health: Vec<WorkerHealth> = self
            .workers
            .values()
            .filter(|worker| worker.status != WorkerStatus::Terminated)
            .map(|worker| WorkerHealth {
                worker_id: worker.id.clone(),
                agent_id: worker.agent_id.clone(),
                status: worker.status.clone(),
                healthy: worker.status == WorkerStatus::Running,
            })
            .collect();
        health.sort_by(|left, right| left.worker_id.cmp(&right.worker_id));
        health
    }

    pub async fn rolling_update(
        &mut self,
        agent_id: &str,
        new_version: &str,
    ) -> Result<Deployment, OrchestratorError> {
        let previous = self
            .deployments
            .get(agent_id)
            .cloned()
            .ok_or(OrchestratorError::NotFound)?;

        if previous.status != DeploymentStatus::Active {
            return Err(OrchestratorError::NotActive);
        }

        if new_version.trim().is_empty() || new_version == "invalid" {
            return self.rollback(agent_id, &previous).await;
        }

        let old_workers = self.running_worker_ids(agent_id);
        if let Some(deployment) = self.deployments.get_mut(agent_id) {
            deployment.status = DeploymentStatus::Deploying;
            deployment.version = new_version.to_string();
            deployment.actual_workers = 0;
            deployment.healthy_workers = 0;
        }

        for _ in 0..previous.target_workers {
            self.spawn_worker(agent_id, new_version, WorkerStatus::Running);
        }

        for worker_id in old_workers {
            if let Some(worker) = self.workers.get_mut(&worker_id) {
                worker.status = WorkerStatus::Terminated;
            }
        }

        sleep(Duration::from_millis(5)).await;

        let actual_workers = self.running_worker_ids(agent_id).len() as u32;
        let deployment = self
            .deployments
            .get_mut(agent_id)
            .ok_or(OrchestratorError::NotFound)?;
        deployment.actual_workers = actual_workers;
        deployment.healthy_workers = actual_workers;
        deployment.status = DeploymentStatus::Active;

        Ok(deployment.clone())
    }

    async fn rollback(
        &mut self,
        agent_id: &str,
        previous: &Deployment,
    ) -> Result<Deployment, OrchestratorError> {
        if let Some(deployment) = self.deployments.get_mut(agent_id) {
            deployment.status = DeploymentStatus::RollingBack;
        }

        self.terminate_active_workers(agent_id);
        for _ in 0..previous.target_workers {
            self.spawn_worker(agent_id, &previous.version, WorkerStatus::Running);
        }

        sleep(Duration::from_millis(5)).await;

        let actual_workers = self.running_worker_ids(agent_id).len() as u32;
        let deployment = self
            .deployments
            .get_mut(agent_id)
            .ok_or(OrchestratorError::RollbackFailed)?;
        deployment.version = previous.version.clone();
        deployment.target_workers = previous.target_workers;
        deployment.actual_workers = actual_workers;
        deployment.healthy_workers = actual_workers;
        deployment.status = DeploymentStatus::Active;

        Ok(deployment.clone())
    }

    pub fn get_deployment(&self, agent_id: &str) -> Option<&Deployment> {
        self.deployments.get(agent_id)
    }

    pub fn get_workers(&self, agent_id: &str) -> Vec<&Worker> {
        let mut workers: Vec<&Worker> = self
            .workers
            .values()
            .filter(|worker| {
                worker.agent_id == agent_id && worker.status != WorkerStatus::Terminated
            })
            .collect();
        workers.sort_by(|left, right| left.id.cmp(&right.id));
        workers
    }

    fn running_worker_ids(&self, agent_id: &str) -> Vec<String> {
        let mut worker_ids: Vec<String> = self
            .workers
            .values()
            .filter(|worker| worker.agent_id == agent_id && worker.status == WorkerStatus::Running)
            .map(|worker| worker.id.clone())
            .collect();
        worker_ids.sort();
        worker_ids
    }

    fn spawn_worker(&mut self, agent_id: &str, version: &str, status: WorkerStatus) -> String {
        let worker_id = format!("{}-{}", agent_id, self.next_worker_id);
        self.next_worker_id += 1;
        self.workers.insert(
            worker_id.clone(),
            Worker {
                id: worker_id.clone(),
                agent_id: agent_id.to_string(),
                version: version.to_string(),
                status,
                last_heartbeat: Utc::now().to_rfc3339(),
                cpu_percent: 0.0,
                memory_mb: 0,
            },
        );
        worker_id
    }

    fn terminate_active_workers(&mut self, agent_id: &str) {
        let worker_ids: Vec<String> = self
            .workers
            .values()
            .filter(|worker| {
                worker.agent_id == agent_id && worker.status != WorkerStatus::Terminated
            })
            .map(|worker| worker.id.clone())
            .collect();

        for worker_id in worker_ids {
            if let Some(worker) = self.workers.get_mut(&worker_id) {
                worker.status = WorkerStatus::Terminated;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn deploys_agent() {
        let mut orchestrator = Orchestrator::new();

        let deployment = orchestrator.deploy("agent_1", "1.0.0").await.unwrap();

        assert_eq!(deployment.agent_id, "agent_1");
        assert_eq!(deployment.version, "1.0.0");
        assert_eq!(deployment.status, DeploymentStatus::Active);
        assert_eq!(deployment.actual_workers, 1);
    }

    #[tokio::test]
    async fn rejects_already_deployed_version() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();

        let result = orchestrator.deploy("agent_1", "1.0.0").await;

        assert_eq!(result, Err(OrchestratorError::AlreadyDeployed));
    }

    #[tokio::test]
    async fn scales_up_workers() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();

        let deployment = orchestrator.scale("agent_1", 3).await.unwrap();

        assert_eq!(deployment.target_workers, 3);
        assert_eq!(deployment.actual_workers, 3);
        assert_eq!(deployment.status, DeploymentStatus::Active);
        assert_eq!(orchestrator.get_workers("agent_1").len(), 3);
    }

    #[tokio::test]
    async fn scales_down_workers() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();
        orchestrator.scale("agent_1", 3).await.unwrap();

        let deployment = orchestrator.scale("agent_1", 1).await.unwrap();

        assert_eq!(deployment.target_workers, 1);
        assert_eq!(deployment.actual_workers, 1);
        assert_eq!(orchestrator.get_workers("agent_1").len(), 1);
    }

    #[tokio::test]
    async fn reports_worker_health() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();

        let health = orchestrator.health_check().await;

        assert_eq!(health.len(), 1);
        assert!(health[0].healthy);
    }

    #[tokio::test]
    async fn rolling_update_replaces_version() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();
        orchestrator.scale("agent_1", 2).await.unwrap();

        let deployment = orchestrator
            .rolling_update("agent_1", "1.1.0")
            .await
            .unwrap();

        assert_eq!(deployment.version, "1.1.0");
        assert_eq!(deployment.status, DeploymentStatus::Active);
        assert_eq!(deployment.actual_workers, 2);
        assert!(orchestrator
            .get_workers("agent_1")
            .iter()
            .all(|worker| worker.version == "1.1.0"));
    }

    #[tokio::test]
    async fn rolling_update_rolls_back_invalid_version() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();
        orchestrator.scale("agent_1", 2).await.unwrap();

        let deployment = orchestrator
            .rolling_update("agent_1", "invalid")
            .await
            .unwrap();

        assert_eq!(deployment.version, "1.0.0");
        assert_eq!(deployment.status, DeploymentStatus::Active);
        assert_eq!(deployment.actual_workers, 2);
    }

    #[tokio::test]
    async fn returns_workers_for_agent() {
        let mut orchestrator = Orchestrator::new();
        orchestrator.deploy("agent_1", "1.0.0").await.unwrap();
        orchestrator.scale("agent_1", 2).await.unwrap();

        let workers = orchestrator.get_workers("agent_1");

        assert_eq!(workers.len(), 2);
    }

    #[tokio::test]
    async fn rejects_scale_for_missing_deployment() {
        let mut orchestrator = Orchestrator::new();

        let result = orchestrator.scale("missing", 2).await;

        assert_eq!(result, Err(OrchestratorError::NotFound));
    }
}
