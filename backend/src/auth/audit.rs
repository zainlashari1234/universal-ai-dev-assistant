use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub ip_address: String,
    pub user_agent: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub outcome: AuditOutcome,
    pub details: HashMap<String, serde_json::Value>,
    pub risk_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    SystemConfiguration,
    SecurityEvent,
    AgentExecution,
    CodeGeneration,
    PolicyViolation,
    AdminAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Partial,
    Blocked,
}

#[async_trait::async_trait]
pub trait AuditService: Send + Sync {
    async fn log_event(&self, event: AuditEvent) -> Result<()>;
    async fn log_authentication(&self, auth_context: &AuthContext) -> Result<()>;
    async fn log_authorization_failure(&self, user_id: Uuid, action: &str, resource: &str) -> Result<()>;
    async fn log_agent_execution(&self, user_id: Uuid, plan_id: Uuid, outcome: AuditOutcome) -> Result<()>;
    async fn log_security_event(&self, event_type: SecurityEventType, details: HashMap<String, serde_json::Value>) -> Result<()>;
    async fn search_events(&self, criteria: AuditSearchCriteria) -> Result<Vec<AuditEvent>>;
    async fn get_user_activity(&self, user_id: Uuid, days: u32) -> Result<Vec<AuditEvent>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    SuspiciousLogin,
    MultipleFailedLogins,
    PrivilegeEscalation,
    UnauthorizedAccess,
    DataExfiltration,
    PolicyViolation,
    AnomalousActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSearchCriteria {
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub outcomes: Option<Vec<AuditOutcome>>,
    pub resource_type: Option<String>,
    pub action: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DatabaseAuditService {
    // In a real implementation, this would have a database connection
    // For now, we'll use in-memory storage for demonstration
    events: std::sync::Arc<tokio::sync::RwLock<Vec<AuditEvent>>>,
}

impl DatabaseAuditService {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl AuditService for DatabaseAuditService {
    async fn log_event(&self, event: AuditEvent) -> Result<()> {
        info!("Audit Event: {} - {} - {}", event.event_type, event.action, event.outcome);
        
        // In production, this would write to a database
        let mut events = self.events.write().await;
        events.push(event);
        
        // Keep only last 10000 events in memory
        if events.len() > 10000 {
            events.remove(0);
        }
        
        Ok(())
    }

    async fn log_authentication(&self, auth_context: &AuthContext) -> Result<()> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authentication,
            user_id: Some(auth_context.user.id),
            session_id: Some(auth_context.session_id),
            organization_id: auth_context.user.organization_id,
            ip_address: auth_context.ip_address.clone(),
            user_agent: auth_context.user_agent.clone(),
            resource_type: None,
            resource_id: None,
            action: "login".to_string(),
            outcome: AuditOutcome::Success,
            details: [
                ("email".to_string(), serde_json::Value::String(auth_context.user.email.clone())),
                ("roles".to_string(), serde_json::Value::Array(
                    auth_context.user.roles.iter()
                        .map(|r| serde_json::Value::String(r.name.clone()))
                        .collect()
                )),
            ].into(),
            risk_score: None,
        };

        self.log_event(event).await
    }

    async fn log_authorization_failure(&self, user_id: Uuid, action: &str, resource: &str) -> Result<()> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authorization,
            user_id: Some(user_id),
            session_id: None,
            organization_id: Uuid::new_v4(), // Would be retrieved from user context
            ip_address: "unknown".to_string(),
            user_agent: "unknown".to_string(),
            resource_type: Some("api_endpoint".to_string()),
            resource_id: Some(resource.to_string()),
            action: action.to_string(),
            outcome: AuditOutcome::Blocked,
            details: [
                ("reason".to_string(), serde_json::Value::String("insufficient_permissions".to_string())),
            ].into(),
            risk_score: Some(0.7), // Authorization failures are medium risk
        };

        self.log_event(event).await
    }

    async fn log_agent_execution(&self, user_id: Uuid, plan_id: Uuid, outcome: AuditOutcome) -> Result<()> {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::AgentExecution,
            user_id: Some(user_id),
            session_id: None,
            organization_id: Uuid::new_v4(), // Would be retrieved from user context
            ip_address: "system".to_string(),
            user_agent: "agent_orchestrator".to_string(),
            resource_type: Some("execution_plan".to_string()),
            resource_id: Some(plan_id.to_string()),
            action: "execute_plan".to_string(),
            outcome,
            details: [
                ("plan_id".to_string(), serde_json::Value::String(plan_id.to_string())),
            ].into(),
            risk_score: match outcome {
                AuditOutcome::Success => Some(0.1),
                AuditOutcome::Failure => Some(0.5),
                AuditOutcome::Blocked => Some(0.8),
                AuditOutcome::Partial => Some(0.3),
            },
        };

        self.log_event(event).await
    }

    async fn log_security_event(&self, event_type: SecurityEventType, details: HashMap<String, serde_json::Value>) -> Result<()> {
        let risk_score = match event_type {
            SecurityEventType::SuspiciousLogin => 0.6,
            SecurityEventType::MultipleFailedLogins => 0.7,
            SecurityEventType::PrivilegeEscalation => 0.9,
            SecurityEventType::UnauthorizedAccess => 0.8,
            SecurityEventType::DataExfiltration => 0.95,
            SecurityEventType::PolicyViolation => 0.5,
            SecurityEventType::AnomalousActivity => 0.4,
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecurityEvent,
            user_id: details.get("user_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok()),
            session_id: details.get("session_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok()),
            organization_id: details.get("organization_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4),
            ip_address: details.get("ip_address")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            user_agent: details.get("user_agent")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            resource_type: None,
            resource_id: None,
            action: format!("security_event_{:?}", event_type),
            outcome: AuditOutcome::Blocked,
            details,
            risk_score: Some(risk_score),
        };

        warn!("Security Event: {:?} - Risk Score: {}", event_type, risk_score);
        self.log_event(event).await
    }

    async fn search_events(&self, criteria: AuditSearchCriteria) -> Result<Vec<AuditEvent>> {
        let events = self.events.read().await;
        let mut filtered_events: Vec<AuditEvent> = events.iter()
            .filter(|event| {
                // Filter by time range
                if let Some(start_time) = criteria.start_time {
                    if event.timestamp < start_time {
                        return false;
                    }
                }
                if let Some(end_time) = criteria.end_time {
                    if event.timestamp > end_time {
                        return false;
                    }
                }

                // Filter by user
                if let Some(user_id) = criteria.user_id {
                    if event.user_id != Some(user_id) {
                        return false;
                    }
                }

                // Filter by organization
                if let Some(org_id) = criteria.organization_id {
                    if event.organization_id != org_id {
                        return false;
                    }
                }

                // Filter by event types
                if let Some(ref event_types) = criteria.event_types {
                    if !event_types.contains(&event.event_type) {
                        return false;
                    }
                }

                // Filter by outcomes
                if let Some(ref outcomes) = criteria.outcomes {
                    if !outcomes.contains(&event.outcome) {
                        return false;
                    }
                }

                // Filter by resource type
                if let Some(ref resource_type) = criteria.resource_type {
                    if event.resource_type.as_ref() != Some(resource_type) {
                        return false;
                    }
                }

                // Filter by action
                if let Some(ref action) = criteria.action {
                    if &event.action != action {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by timestamp (newest first)
        filtered_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = criteria.offset.unwrap_or(0);
        let limit = criteria.limit.unwrap_or(100);
        
        let end_index = std::cmp::min(offset + limit, filtered_events.len());
        if offset < filtered_events.len() {
            Ok(filtered_events[offset..end_index].to_vec())
        } else {
            Ok(vec![])
        }
    }

    async fn get_user_activity(&self, user_id: Uuid, days: u32) -> Result<Vec<AuditEvent>> {
        let start_time = chrono::Utc::now() - chrono::Duration::days(days as i64);
        
        let criteria = AuditSearchCriteria {
            start_time: Some(start_time),
            end_time: None,
            user_id: Some(user_id),
            organization_id: None,
            event_types: None,
            outcomes: None,
            resource_type: None,
            action: None,
            limit: Some(1000),
            offset: None,
        };

        self.search_events(criteria).await
    }
}

// Helper functions for creating audit events
impl AuditEvent {
    pub fn new_authentication_success(auth_context: &AuthContext) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authentication,
            user_id: Some(auth_context.user.id),
            session_id: Some(auth_context.session_id),
            organization_id: auth_context.user.organization_id,
            ip_address: auth_context.ip_address.clone(),
            user_agent: auth_context.user_agent.clone(),
            resource_type: None,
            resource_id: None,
            action: "authentication_success".to_string(),
            outcome: AuditOutcome::Success,
            details: HashMap::new(),
            risk_score: Some(0.1),
        }
    }

    pub fn new_code_generation(user_id: Uuid, org_id: Uuid, language: &str, success: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::CodeGeneration,
            user_id: Some(user_id),
            session_id: None,
            organization_id: org_id,
            ip_address: "system".to_string(),
            user_agent: "code_generator".to_string(),
            resource_type: Some("code".to_string()),
            resource_id: None,
            action: "generate_code".to_string(),
            outcome: if success { AuditOutcome::Success } else { AuditOutcome::Failure },
            details: [
                ("language".to_string(), serde_json::Value::String(language.to_string())),
            ].into(),
            risk_score: Some(if success { 0.2 } else { 0.4 }),
        }
    }

    pub fn new_policy_violation(user_id: Uuid, org_id: Uuid, policy: &str, details: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::PolicyViolation,
            user_id: Some(user_id),
            session_id: None,
            organization_id: org_id,
            ip_address: "system".to_string(),
            user_agent: "policy_engine".to_string(),
            resource_type: Some("policy".to_string()),
            resource_id: Some(policy.to_string()),
            action: "policy_violation".to_string(),
            outcome: AuditOutcome::Blocked,
            details,
            risk_score: Some(0.7),
        }
    }
}

// Audit event serialization for different formats
impl AuditEvent {
    pub fn to_siem_format(&self) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.timestamp.to_rfc3339(),
            "event_id": self.id,
            "event_type": format!("{:?}", self.event_type),
            "user_id": self.user_id,
            "organization_id": self.organization_id,
            "source_ip": self.ip_address,
            "user_agent": self.user_agent,
            "action": self.action,
            "outcome": format!("{:?}", self.outcome),
            "risk_score": self.risk_score,
            "details": self.details
        })
    }

    pub fn to_compliance_format(&self) -> serde_json::Value {
        serde_json::json!({
            "audit_id": self.id,
            "timestamp": self.timestamp.to_rfc3339(),
            "actor": {
                "user_id": self.user_id,
                "session_id": self.session_id,
                "ip_address": self.ip_address
            },
            "action": {
                "type": format!("{:?}", self.event_type),
                "name": self.action,
                "outcome": format!("{:?}", self.outcome)
            },
            "resource": {
                "type": self.resource_type,
                "id": self.resource_id
            },
            "context": {
                "organization_id": self.organization_id,
                "risk_score": self.risk_score,
                "details": self.details
            }
        })
    }
}