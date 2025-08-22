use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub subscription_tier: SubscriptionTier,
    pub max_users: Option<i32>,
    pub max_api_calls_per_month: Option<i64>,
    pub settings: OrganizationSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Free,
    Pro,
    Enterprise,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    pub allow_user_registration: bool,
    pub require_email_verification: bool,
    pub enforce_2fa: bool,
    pub allowed_domains: Vec<String>,
    pub default_user_role: String,
    pub api_rate_limits: ApiRateLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub concurrent_requests: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub subscription_tier: SubscriptionTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<OrganizationSettings>,
}

pub struct OrganizationService {
    pool: PgPool,
}

impl OrganizationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_organization(
        &self,
        owner_id: Uuid,
        request: CreateOrganizationRequest,
    ) -> Result<Organization> {
        // Check if slug is available
        let existing = sqlx::query!(
            "SELECT id FROM organizations WHERE slug = $1",
            request.slug
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!("Organization slug already exists"));
        }

        // Default settings
        let default_settings = OrganizationSettings {
            allow_user_registration: true,
            require_email_verification: false,
            enforce_2fa: false,
            allowed_domains: vec![],
            default_user_role: "developer".to_string(),
            api_rate_limits: ApiRateLimits {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
                concurrent_requests: 10,
            },
        };

        let (max_users, max_api_calls) = match request.subscription_tier {
            SubscriptionTier::Free => (Some(5), Some(1000)),
            SubscriptionTier::Pro => (Some(50), Some(100000)),
            SubscriptionTier::Enterprise => (Some(500), Some(1000000)),
            SubscriptionTier::Custom => (None, None),
        };

        let org = sqlx::query!(
            r#"
            INSERT INTO organizations (
                name, slug, description, owner_id, subscription_tier, 
                max_users, max_api_calls_per_month, settings
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, slug, description, owner_id, subscription_tier, 
                     max_users, max_api_calls_per_month, settings, created_at, updated_at
            "#,
            request.name,
            request.slug,
            request.description,
            owner_id,
            serde_json::to_string(&request.subscription_tier)?,
            max_users,
            max_api_calls,
            serde_json::to_string(&default_settings)?
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Organization {
            id: org.id,
            name: org.name,
            slug: org.slug,
            description: org.description,
            owner_id: org.owner_id,
            subscription_tier: serde_json::from_str(&org.subscription_tier.unwrap_or_default())?,
            max_users: org.max_users,
            max_api_calls_per_month: org.max_api_calls_per_month,
            settings: serde_json::from_str(&org.settings.unwrap_or_default())?,
            created_at: org.created_at,
            updated_at: org.updated_at,
        })
    }

    pub async fn get_organization(&self, org_id: Uuid) -> Result<Option<Organization>> {
        let org = sqlx::query!(
            r#"
            SELECT id, name, slug, description, owner_id, subscription_tier,
                   max_users, max_api_calls_per_month, settings, created_at, updated_at
            FROM organizations
            WHERE id = $1
            "#,
            org_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(org) = org {
            Ok(Some(Organization {
                id: org.id,
                name: org.name,
                slug: org.slug,
                description: org.description,
                owner_id: org.owner_id,
                subscription_tier: serde_json::from_str(&org.subscription_tier.unwrap_or_default())?,
                max_users: org.max_users,
                max_api_calls_per_month: org.max_api_calls_per_month,
                settings: serde_json::from_str(&org.settings.unwrap_or_default())?,
                created_at: org.created_at,
                updated_at: org.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_organizations(&self, user_id: Uuid) -> Result<Vec<Organization>> {
        let orgs = sqlx::query!(
            r#"
            SELECT o.id, o.name, o.slug, o.description, o.owner_id, o.subscription_tier,
                   o.max_users, o.max_api_calls_per_month, o.settings, o.created_at, o.updated_at
            FROM organizations o
            LEFT JOIN organization_members om ON o.id = om.organization_id
            WHERE o.owner_id = $1 OR om.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut organizations = Vec::new();
        for org in orgs {
            organizations.push(Organization {
                id: org.id,
                name: org.name,
                slug: org.slug,
                description: org.description,
                owner_id: org.owner_id,
                subscription_tier: serde_json::from_str(&org.subscription_tier.unwrap_or_default())?,
                max_users: org.max_users,
                max_api_calls_per_month: org.max_api_calls_per_month,
                settings: serde_json::from_str(&org.settings.unwrap_or_default())?,
                created_at: org.created_at,
                updated_at: org.updated_at,
            });
        }

        Ok(organizations)
    }

    pub async fn update_organization(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        request: UpdateOrganizationRequest,
    ) -> Result<Organization> {
        // Check if user is owner
        let org = self.get_organization(org_id).await?;
        let org = org.ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        if org.owner_id != user_id {
            return Err(anyhow::anyhow!("Only organization owner can update settings"));
        }

        let updated_org = sqlx::query!(
            r#"
            UPDATE organizations
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                settings = COALESCE($4, settings),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, slug, description, owner_id, subscription_tier,
                     max_users, max_api_calls_per_month, settings, created_at, updated_at
            "#,
            org_id,
            request.name,
            request.description,
            request.settings.map(|s| serde_json::to_string(&s)).transpose()?
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Organization {
            id: updated_org.id,
            name: updated_org.name,
            slug: updated_org.slug,
            description: updated_org.description,
            owner_id: updated_org.owner_id,
            subscription_tier: serde_json::from_str(&updated_org.subscription_tier.unwrap_or_default())?,
            max_users: updated_org.max_users,
            max_api_calls_per_month: updated_org.max_api_calls_per_month,
            settings: serde_json::from_str(&updated_org.settings.unwrap_or_default())?,
            created_at: updated_org.created_at,
            updated_at: updated_org.updated_at,
        })
    }

    pub async fn add_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
        invited_by: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO organization_members (organization_id, user_id, role, invited_by)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (organization_id, user_id) DO UPDATE SET
                role = EXCLUDED.role,
                updated_at = NOW()
            "#,
            org_id,
            user_id,
            role,
            invited_by
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_member(&self, org_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM organization_members WHERE organization_id = $1 AND user_id = $2",
            org_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_default_organization(&self, user_id: Uuid) -> Result<Option<Organization>> {
        // First try to get user's owned organization
        let owned_org = sqlx::query!(
            r#"
            SELECT id, name, slug, description, owner_id, subscription_tier,
                   max_users, max_api_calls_per_month, settings, created_at, updated_at
            FROM organizations
            WHERE owner_id = $1
            ORDER BY created_at ASC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(org) = owned_org {
            return Ok(Some(Organization {
                id: org.id,
                name: org.name,
                slug: org.slug,
                description: org.description,
                owner_id: org.owner_id,
                subscription_tier: serde_json::from_str(&org.subscription_tier.unwrap_or_default())?,
                max_users: org.max_users,
                max_api_calls_per_month: org.max_api_calls_per_month,
                settings: serde_json::from_str(&org.settings.unwrap_or_default())?,
                created_at: org.created_at,
                updated_at: org.updated_at,
            }));
        }

        // If no owned organization, get first organization user is member of
        let member_org = sqlx::query!(
            r#"
            SELECT o.id, o.name, o.slug, o.description, o.owner_id, o.subscription_tier,
                   o.max_users, o.max_api_calls_per_month, o.settings, o.created_at, o.updated_at
            FROM organizations o
            JOIN organization_members om ON o.id = om.organization_id
            WHERE om.user_id = $1
            ORDER BY om.created_at ASC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(org) = member_org {
            Ok(Some(Organization {
                id: org.id,
                name: org.name,
                slug: org.slug,
                description: org.description,
                owner_id: org.owner_id,
                subscription_tier: serde_json::from_str(&org.subscription_tier.unwrap_or_default())?,
                max_users: org.max_users,
                max_api_calls_per_month: org.max_api_calls_per_month,
                settings: serde_json::from_str(&org.settings.unwrap_or_default())?,
                created_at: org.created_at,
                updated_at: org.updated_at,
            }))
        } else {
            Ok(None)
        }
    }
}