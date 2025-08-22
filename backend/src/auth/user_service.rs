use super::*;
use anyhow::{Result, anyhow};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub full_name: Option<String>,
    pub is_active: bool,
    pub is_verified: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub full_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub struct UserService {
    pool: PgPool,
    jwt_manager: JwtManager,
}

impl UserService {
    pub fn new(pool: PgPool, jwt_manager: JwtManager) -> Self {
        Self { pool, jwt_manager }
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest) -> Result<UserResponse> {
        // Validate email format
        if !self.is_valid_email(&request.email) {
            return Err(anyhow!("Invalid email format"));
        }

        // Validate password strength
        if !self.is_strong_password(&request.password) {
            return Err(anyhow!("Password must be at least 8 characters with uppercase, lowercase, number, and special character"));
        }

        // Check if user already exists
        let existing_user = sqlx::query!(
            "SELECT id FROM users WHERE email = $1 OR username = $2",
            request.email,
            request.username
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing_user.is_some() {
            return Err(anyhow!("User with this email or username already exists"));
        }

        // Hash password
        let password_hash = hash(&request.password, DEFAULT_COST)?;

        // Insert user
        let user = sqlx::query!(
            r#"
            INSERT INTO users (email, username, password_hash, full_name)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, username, full_name, is_active, is_verified, last_login_at, created_at
            "#,
            request.email,
            request.username,
            password_hash,
            request.full_name
        )
        .fetch_one(&self.pool)
        .await?;

        // Create default user preferences
        sqlx::query!(
            "INSERT INTO user_preferences (user_id) VALUES ($1)",
            user.id
        )
        .execute(&self.pool)
        .await?;

        Ok(UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            full_name: user.full_name,
            is_active: user.is_active,
            is_verified: user.is_verified,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        })
    }

    /// Login user and create session
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        // Get user by email
        let user_row = sqlx::query!(
            r#"
            SELECT id, email, username, password_hash, full_name, is_active, is_verified, last_login_at, created_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#,
            request.email
        )
        .fetch_optional(&self.pool)
        .await?;

        let user_row = user_row.ok_or_else(|| anyhow!("Invalid email or password"))?;

        // Verify password
        if !verify(&request.password, &user_row.password_hash)? {
            return Err(anyhow!("Invalid email or password"));
        }

        // Create session
        let session_id = Uuid::new_v4();
        let session_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        sqlx::query!(
            r#"
            INSERT INTO sessions (id, user_id, session_token, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
            session_id,
            user_row.id,
            session_token,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        // Update last login
        sqlx::query!(
            "UPDATE users SET last_login_at = NOW() WHERE id = $1",
            user_row.id
        )
        .execute(&self.pool)
        .await?;

        // Create user object for JWT
        let user = User {
            id: user_row.id,
            email: user_row.email.clone(),
            name: user_row.full_name.unwrap_or_else(|| user_row.username.clone()),
            organization_id: self.get_or_create_default_organization(user_row.id).await?,
            roles: self.get_user_roles(user_row.id).await?,
            permissions: self.get_user_permissions(user_row.id).await?,
            created_at: user_row.created_at,
            last_login: user_row.last_login_at,
            is_active: user_row.is_active,
        };

        // Generate JWT tokens
        let tokens = self.jwt_manager.generate_token_pair(&user, session_id)?;

        let user_response = UserResponse {
            id: user_row.id,
            email: user_row.email,
            username: user_row.username,
            full_name: user_row.full_name,
            is_active: user_row.is_active,
            is_verified: user_row.is_verified,
            last_login_at: Some(Utc::now()),
            created_at: user_row.created_at,
        };

        Ok(LoginResponse {
            user: user_response,
            tokens,
            session_id,
        })
    }

    /// Logout user and invalidate session
    pub async fn logout(&self, session_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE sessions SET is_active = false WHERE id = $1",
            session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<UserResponse>> {
        let user = sqlx::query!(
            r#"
            SELECT id, email, username, full_name, is_active, is_verified, last_login_at, created_at
            FROM users
            WHERE id = $1 AND is_active = true
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user) = user {
            Ok(Some(UserResponse {
                id: user.id,
                email: user.email,
                username: user.username,
                full_name: user.full_name,
                is_active: user.is_active,
                is_verified: user.is_verified,
                last_login_at: user.last_login_at,
                created_at: user.created_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update user profile
    pub async fn update_user(&self, user_id: Uuid, request: UpdateUserRequest) -> Result<UserResponse> {
        // Check if username is taken (if provided)
        if let Some(ref username) = request.username {
            let existing = sqlx::query!(
                "SELECT id FROM users WHERE username = $1 AND id != $2",
                username,
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            if existing.is_some() {
                return Err(anyhow!("Username already taken"));
            }
        }

        // Update user
        let user = sqlx::query!(
            r#"
            UPDATE users
            SET 
                full_name = COALESCE($2, full_name),
                username = COALESCE($3, username),
                updated_at = NOW()
            WHERE id = $1 AND is_active = true
            RETURNING id, email, username, full_name, is_active, is_verified, last_login_at, created_at
            "#,
            user_id,
            request.full_name,
            request.username
        )
        .fetch_optional(&self.pool)
        .await?;

        let user = user.ok_or_else(|| anyhow!("User not found"))?;

        Ok(UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            full_name: user.full_name,
            is_active: user.is_active,
            is_verified: user.is_verified,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        })
    }

    /// Change user password
    pub async fn change_password(&self, user_id: Uuid, request: ChangePasswordRequest) -> Result<()> {
        // Get current password hash
        let user = sqlx::query!(
            "SELECT password_hash FROM users WHERE id = $1 AND is_active = true",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let user = user.ok_or_else(|| anyhow!("User not found"))?;

        // Verify current password
        if !verify(&request.current_password, &user.password_hash)? {
            return Err(anyhow!("Current password is incorrect"));
        }

        // Validate new password strength
        if !self.is_strong_password(&request.new_password) {
            return Err(anyhow!("New password must be at least 8 characters with uppercase, lowercase, number, and special character"));
        }

        // Hash new password
        let new_password_hash = hash(&request.new_password, DEFAULT_COST)?;

        // Update password
        sqlx::query!(
            "UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1",
            user_id,
            new_password_hash
        )
        .execute(&self.pool)
        .await?;

        // Invalidate all sessions for security
        sqlx::query!(
            "UPDATE sessions SET is_active = false WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Validate session
    pub async fn validate_session(&self, session_id: Uuid) -> Result<bool> {
        let session = sqlx::query!(
            r#"
            SELECT expires_at, is_active
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(session) = session {
            Ok(session.is_active && session.expires_at > Utc::now())
        } else {
            Ok(false)
        }
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let result = sqlx::query!("SELECT cleanup_expired_sessions()")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.cleanup_expired_sessions.unwrap_or(0) as u64)
    }

    // Helper methods
    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5
    }

    fn is_strong_password(&self, password: &str) -> bool {
        password.len() >= 8
            && password.chars().any(|c| c.is_uppercase())
            && password.chars().any(|c| c.is_lowercase())
            && password.chars().any(|c| c.is_numeric())
            && password.chars().any(|c| !c.is_alphanumeric())
    }

    async fn get_or_create_default_organization(&self, user_id: Uuid) -> Result<Uuid> {
        // Try to get existing organization
        let existing_org = sqlx::query!(
            "SELECT get_user_organization($1) as org_id",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(org_id) = existing_org.org_id {
            return Ok(org_id);
        }

        // Create default personal organization
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, owner_id, subscription_tier)
            VALUES ($1, $2, $3, $4, 'free')
            "#,
            org_id,
            format!("{}'s Organization", user_id),
            format!("user-{}", user_id.to_string()[..8].to_lowercase()),
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(org_id)
    }

    async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<Role>> {
        // For now, return default developer role
        // TODO: Implement proper role management
        Ok(vec![Role::developer_role(Uuid::new_v4())])
    }

    async fn get_user_permissions(&self, user_id: Uuid) -> Result<Vec<Permission>> {
        // For now, return default permissions
        // TODO: Implement proper permission management based on roles
        Ok(vec![
            Permission::ApiAccess,
            Permission::CreatePlan,
            Permission::ReadProject,
            Permission::WriteProject,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let service = UserService::new(
            unsafe { std::mem::zeroed() },
            JwtManager::new("test", "test".to_string())
        );

        assert!(service.is_valid_email("test@example.com"));
        assert!(!service.is_valid_email("invalid-email"));
        assert!(!service.is_valid_email("test@"));
        assert!(!service.is_valid_email("@example.com"));
    }

    #[test]
    fn test_password_strength() {
        let service = UserService::new(
            unsafe { std::mem::zeroed() },
            JwtManager::new("test", "test".to_string())
        );

        assert!(service.is_strong_password("StrongPass123!"));
        assert!(!service.is_strong_password("weak"));
        assert!(!service.is_strong_password("NoNumbers!"));
        assert!(!service.is_strong_password("nonumbers123"));
        assert!(!service.is_strong_password("NoSpecialChars123"));
    }
}