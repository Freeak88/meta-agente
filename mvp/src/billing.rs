#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Plan {
    Free {
        max_executions_per_month: u32,
        max_storage_mb: u32,
        max_agents: u32,
    },
    Pro {
        max_executions_per_month: u32,
        max_storage_gb: u32,
        max_agents: u32,
        priority: bool,
    },
    Enterprise {
        custom: bool,
        sla: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub id: String,
    pub plan: Plan,
    pub credits: f64,
    pub used_executions: u32,
    pub used_storage_mb: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BillingError {
    ExecutionLimitExceeded,
    StorageLimitExceeded,
    InsufficientCredits,
    AgentLimitExceeded,
}

impl BillingError {
    pub fn http_status(&self) -> u16 {
        402
    }
}

pub struct BillingEngine;

impl BillingEngine {
    pub fn can_execute(account: &Account, estimated_cost: Option<f64>) -> Result<(), BillingError> {
        match &account.plan {
            Plan::Free {
                max_executions_per_month,
                ..
            } => {
                if account.used_executions >= *max_executions_per_month {
                    return Err(BillingError::ExecutionLimitExceeded);
                }
            }
            Plan::Pro {
                max_executions_per_month,
                ..
            } => {
                if account.used_executions >= *max_executions_per_month {
                    return Err(BillingError::ExecutionLimitExceeded);
                }
                if let Some(cost) = estimated_cost {
                    if account.credits < cost {
                        return Err(BillingError::InsufficientCredits);
                    }
                }
            }
            Plan::Enterprise { custom, .. } => {
                if let Some(cost) = estimated_cost {
                    if !custom && account.credits < cost {
                        return Err(BillingError::InsufficientCredits);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn charge_execution(account: &mut Account, duration_ms: u64, steps: u32) -> f64 {
        let cost = Self::calculate_cost(duration_ms, steps);
        account.used_executions += 1;

        if matches!(account.plan, Plan::Pro { .. } | Plan::Enterprise { .. }) {
            account.credits -= cost;
        }

        cost
    }

    pub fn charge_storage(account: &Account, bytes: u64) -> Result<(), BillingError> {
        let mb = bytes / 1024 / 1024;

        match &account.plan {
            Plan::Free { max_storage_mb, .. } if mb > *max_storage_mb as u64 => {
                Err(BillingError::StorageLimitExceeded)
            }
            Plan::Pro { max_storage_gb, .. } if mb > *max_storage_gb as u64 * 1024 => {
                Err(BillingError::StorageLimitExceeded)
            }
            _ => Ok(()),
        }
    }

    pub fn can_create_agent(account: &Account, current_agents: u32) -> Result<(), BillingError> {
        let max_agents = match &account.plan {
            Plan::Free { max_agents, .. } | Plan::Pro { max_agents, .. } => *max_agents,
            Plan::Enterprise { .. } => return Ok(()),
        };

        if current_agents >= max_agents {
            Err(BillingError::AgentLimitExceeded)
        } else {
            Ok(())
        }
    }

    pub fn calculate_cost(duration_ms: u64, steps: u32) -> f64 {
        let base = steps as f64 * 0.01;
        let time_multiplier = if duration_ms > 60_000 {
            5.0
        } else if duration_ms > 10_000 {
            2.0
        } else {
            1.0
        };

        base * time_multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_account() -> Account {
        Account {
            id: "free_001".to_string(),
            plan: Plan::Free {
                max_executions_per_month: 100,
                max_storage_mb: 100,
                max_agents: 3,
            },
            credits: 0.0,
            used_executions: 0,
            used_storage_mb: 0,
        }
    }

    fn pro_account() -> Account {
        Account {
            id: "pro_001".to_string(),
            plan: Plan::Pro {
                max_executions_per_month: 10_000,
                max_storage_gb: 10,
                max_agents: 50,
                priority: true,
            },
            credits: 100.0,
            used_executions: 0,
            used_storage_mb: 0,
        }
    }

    fn enterprise_account() -> Account {
        Account {
            id: "ent_001".to_string(),
            plan: Plan::Enterprise {
                custom: true,
                sla: "99.99%".to_string(),
            },
            credits: 0.0,
            used_executions: 0,
            used_storage_mb: 0,
        }
    }

    #[test]
    fn free_can_execute_within_limit() {
        assert!(BillingEngine::can_execute(&free_account(), None).is_ok());
    }

    #[test]
    fn free_cannot_execute_over_limit() {
        let mut account = free_account();
        account.used_executions = 100;

        assert_eq!(
            BillingEngine::can_execute(&account, None),
            Err(BillingError::ExecutionLimitExceeded)
        );
    }

    #[test]
    fn pro_can_execute_with_credits() {
        assert!(BillingEngine::can_execute(&pro_account(), Some(1.0)).is_ok());
    }

    #[test]
    fn pro_cannot_execute_without_credits() {
        let mut account = pro_account();
        account.credits = 0.01;

        assert_eq!(
            BillingEngine::can_execute(&account, Some(1.0)),
            Err(BillingError::InsufficientCredits)
        );
    }

    #[test]
    fn enterprise_custom_has_unlimited_executions() {
        assert!(BillingEngine::can_execute(&enterprise_account(), Some(10_000.0)).is_ok());
    }

    #[test]
    fn charge_execution_free_counts_without_spending_credits() {
        let mut account = free_account();
        let cost = BillingEngine::charge_execution(&mut account, 5_000, 5);

        assert!(cost > 0.0);
        assert_eq!(account.used_executions, 1);
        assert_eq!(account.credits, 0.0);
    }

    #[test]
    fn charge_execution_pro_spends_credits() {
        let mut account = pro_account();
        let initial_credits = account.credits;
        let cost = BillingEngine::charge_execution(&mut account, 5_000, 5);

        assert!(cost > 0.0);
        assert_eq!(account.used_executions, 1);
        assert_eq!(account.credits, initial_credits - cost);
    }

    #[test]
    fn slow_executions_cost_more() {
        let cost_fast = BillingEngine::calculate_cost(1_000, 5);
        let cost_slow = BillingEngine::calculate_cost(15_000, 5);

        assert!(cost_slow > cost_fast);
    }

    #[test]
    fn very_slow_executions_cost_even_more() {
        let cost = BillingEngine::calculate_cost(65_000, 5);

        assert_eq!(cost, 0.25);
    }

    #[test]
    fn storage_free_within_limit() {
        assert!(BillingEngine::charge_storage(&free_account(), 50 * 1024 * 1024).is_ok());
    }

    #[test]
    fn storage_free_over_limit() {
        assert_eq!(
            BillingEngine::charge_storage(&free_account(), 150 * 1024 * 1024),
            Err(BillingError::StorageLimitExceeded)
        );
    }

    #[test]
    fn agent_limit_maps_to_payment_required() {
        assert_eq!(
            BillingEngine::can_create_agent(&free_account(), 3),
            Err(BillingError::AgentLimitExceeded)
        );
        assert_eq!(BillingError::AgentLimitExceeded.http_status(), 402);
    }

    #[test]
    fn billing_errors_map_to_402_payment_required() {
        let errors = [
            BillingError::ExecutionLimitExceeded,
            BillingError::StorageLimitExceeded,
            BillingError::InsufficientCredits,
            BillingError::AgentLimitExceeded,
        ];

        for error in errors {
            assert_eq!(error.http_status(), 402);
        }
    }
}
