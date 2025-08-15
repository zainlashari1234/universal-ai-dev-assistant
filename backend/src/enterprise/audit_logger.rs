// Enterprise Audit Logging
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use super::EnterpriseConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub outcome: AuditOutcome,
    pub details: HashMap<String, String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    SystemConfiguration,
    SecurityEvent,
    ComplianceEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Blocked,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

pub struct AuditLogger {
    config: EnterpriseConfig,
    events: RwLock<Vec<AuditEvent>>,
    metrics: RwLock<AuditMetrics>,
}

#[derive(Debug, Default)]
struct AuditMetrics {
    total_events: usize,
    security_events: usize,
    failed_authentications: usize,
    high_risk_events: usize,
}

impl AuditLogger {
    pub async fn new(config: &EnterpriseConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            events: RwLock::new(Vec::new()),
            metrics: RwLock::new(AuditMetrics::default()),
        })
    }
    
    pub async fn start_logging(&self) -> Result<()> {
        info!("Starting enterprise audit logging");
        
        // Start background cleanup task
        let logger = self.clone();
        tokio::spawn(async move {
            logger.cleanup_old_events().await;
        });
        
        Ok(())
    }
    
    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_events += 1;
            
            match event.event_type {
                AuditEventType::SecurityEvent => metrics.security_events += 1,
                AuditEventType::Authentication if matches!(event.outcome, AuditOutcome::Failure) => {
                    metrics.failed_authentications += 1;
                }
                _ => {}
            }
            
            if matches!(event.risk_level, RiskLevel::High | RiskLevel::Critical) {
                metrics.high_risk_events += 1;
            }
        }
        
        // Log high-risk events immediately
        if matches!(event.risk_level, RiskLevel::High | RiskLevel::Critical) {
            warn!("High-risk audit event: {:?}", event);
        }
        
        // Store event
        {
            let mut events = self.events.write().await;
            events.push(event);
        }
        
        Ok(())
    }
    
    pub async fn log_authentication(&self, user_id: &str, success: bool, ip: &str) -> Result<()> {
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            event_type: AuditEventType::Authentication,
            user_id: Some(user_id.to_string()),
            resource: "auth".to_string(),
            action: "login".to_string(),
            outcome: if success { AuditOutcome::Success } else { AuditOutcome::Failure },
            details: HashMap::new(),
            ip_address: ip.to_string(),
            user_agent: None,
            timestamp: chrono::Utc::now(),
            risk_level: if success { RiskLevel::Low } else { RiskLevel::Medium },
        };
        
        self.log_event(event).await
    }
    
    pub async fn log_data_access(&self, user_id: &str, resource: &str, ip: &str) -> Result<()> {
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            event_type: AuditEventType::DataAccess,
            user_id: Some(user_id.to_string()),
            resource: resource.to_string(),
            action: "read".to_string(),
            outcome: AuditOutcome::Success,
            details: HashMap::new(),
            ip_address: ip.to_string(),
            user_agent: None,
            timestamp: chrono::Utc::now(),
            risk_level: RiskLevel::Low,
        };
        
        self.log_event(event).await
    }
    
    pub async fn log_security_event(&self, description: &str, risk_level: RiskLevel, ip: &str) -> Result<()> {
        let mut details = HashMap::new();
        details.insert("description".to_string(), description.to_string());
        
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            event_type: AuditEventType::SecurityEvent,
            user_id: None,
            resource: "system".to_string(),
            action: "security_alert".to_string(),
            outcome: AuditOutcome::Warning,
            details,
            ip_address: ip.to_string(),
            user_agent: None,
            timestamp: chrono::Utc::now(),
            risk_level,
        };
        
        self.log_event(event).await
    }
    
    pub async fn get_events(&self, limit: Option<usize>) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        let limit = limit.unwrap_or(100);
        
        events.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    pub async fn get_request_count(&self) -> Result<usize> {
        let metrics = self.metrics.read().await;
        Ok(metrics.total_events)
    }
    
    pub async fn get_security_events(&self) -> Result<usize> {
        let metrics = self.metrics.read().await;
        Ok(metrics.security_events)
    }
    
    async fn cleanup_old_events(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_hours(24));
        
        loop {
            interval.tick().await;
            
            let cutoff = chrono::Utc::now() - chrono::Duration::days(self.config.data_retention_days as i64);
            
            let mut events = self.events.write().await;
            let initial_count = events.len();
            events.retain(|event| event.timestamp > cutoff);
            let removed = initial_count - events.len();
            
            if removed > 0 {
                info!("Cleaned up {} old audit events", removed);
            }
        }
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            events: RwLock::new(Vec::new()),
            metrics: RwLock::new(AuditMetrics::default()),
        }
    }
}