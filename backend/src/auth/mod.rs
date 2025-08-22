pub mod sso;
pub mod rbac;
pub mod jwt;
pub mod middleware;
pub mod audit;
pub mod api_key_manager;
pub mod user_service;
pub mod handlers;
pub mod organization;
pub mod preferences;
pub mod preferences_handlers;

pub use sso::*;
pub use rbac::*;
pub use jwt::*;
pub use middleware::*;
pub use audit::*;
pub use api_key_manager::*;
pub use user_service::*;
pub use handlers::*;
pub use organization::*;
pub use preferences::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub organization_id: Uuid,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub sso_config: Option<SsoConfig>,
    pub settings: OrganizationSettings,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub organization_id: Uuid,
    pub is_system_role: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    // Agent permissions
    CreatePlan,
    ExecutePlan,
    ViewPlan,
    CancelPlan,
    
    // Code permissions
    GenerateCode,
    ReviewCode,
    ApproveCode,
    DeployCode,
    
    // Security permissions
    ViewSecurityReports,
    OverrideSecurityBlocks,
    ConfigureSecurity,
    
    // Admin permissions
    ManageUsers,
    ManageRoles,
    ManageOrganization,
    ViewAuditLogs,
    ConfigureSystem,
    
    // API permissions
    ApiAccess,
    ApiAdmin,
    
    // Evaluation permissions
    RunEvaluations,
    ViewEvaluations,
    ConfigureEvaluations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    pub max_users: Option<usize>,
    pub max_concurrent_executions: Option<usize>,
    pub allowed_languages: Vec<String>,
    pub security_policy: SecurityPolicy,
    pub audit_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub require_mfa: bool,
    pub session_timeout_minutes: u32,
    pub max_failed_logins: u32,
    pub password_policy: PasswordPolicy,
    pub ip_whitelist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: u32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_symbols: bool,
    pub max_age_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user: User,
    pub session_id: Uuid,
    pub ip_address: String,
    pub user_agent: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub permissions: Vec<Permission>,
}

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            max_users: Some(100),
            max_concurrent_executions: Some(10),
            allowed_languages: vec![
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "rust".to_string(),
            ],
            security_policy: SecurityPolicy::default(),
            audit_retention_days: 365,
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            require_mfa: false,
            session_timeout_minutes: 480, // 8 hours
            max_failed_logins: 5,
            password_policy: PasswordPolicy::default(),
            ip_whitelist: None,
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: false,
            max_age_days: Some(90),
        }
    }
}

impl User {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission) || 
        self.roles.iter().any(|role| role.permissions.contains(permission))
    }

    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }

    pub fn is_admin(&self) -> bool {
        self.has_permission(&Permission::ManageOrganization)
    }

    pub fn can_access_api(&self) -> bool {
        self.has_permission(&Permission::ApiAccess)
    }
}

impl Role {
    pub fn admin_role(organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Admin".to_string(),
            description: "Full administrative access".to_string(),
            permissions: vec![
                Permission::CreatePlan,
                Permission::ExecutePlan,
                Permission::ViewPlan,
                Permission::CancelPlan,
                Permission::GenerateCode,
                Permission::ReviewCode,
                Permission::ApproveCode,
                Permission::DeployCode,
                Permission::ViewSecurityReports,
                Permission::OverrideSecurityBlocks,
                Permission::ConfigureSecurity,
                Permission::ManageUsers,
                Permission::ManageRoles,
                Permission::ManageOrganization,
                Permission::ViewAuditLogs,
                Permission::ConfigureSystem,
                Permission::ApiAccess,
                Permission::ApiAdmin,
                Permission::RunEvaluations,
                Permission::ViewEvaluations,
                Permission::ConfigureEvaluations,
            ],
            organization_id,
            is_system_role: true,
        }
    }

    pub fn developer_role(organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Developer".to_string(),
            description: "Standard developer access".to_string(),
            permissions: vec![
                Permission::CreatePlan,
                Permission::ExecutePlan,
                Permission::ViewPlan,
                Permission::GenerateCode,
                Permission::ReviewCode,
                Permission::ViewSecurityReports,
                Permission::ApiAccess,
                Permission::ViewEvaluations,
            ],
            organization_id,
            is_system_role: true,
        }
    }

    pub fn viewer_role(organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Viewer".to_string(),
            description: "Read-only access".to_string(),
            permissions: vec![
                Permission::ViewPlan,
                Permission::ViewSecurityReports,
                Permission::ApiAccess,
                Permission::ViewEvaluations,
            ],
            organization_id,
            is_system_role: true,
        }
    }

    pub fn auditor_role(organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Auditor".to_string(),
            description: "Audit and compliance access".to_string(),
            permissions: vec![
                Permission::ViewPlan,
                Permission::ViewSecurityReports,
                Permission::ViewAuditLogs,
                Permission::ApiAccess,
                Permission::ViewEvaluations,
            ],
            organization_id,
            is_system_role: true,
        }
    }
}