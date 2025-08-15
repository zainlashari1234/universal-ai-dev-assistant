use super::*;
use anyhow::Result;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    pub provider: SsoProvider,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SsoProvider {
    Oidc,
    Saml,
    AzureAd,
    GoogleWorkspace,
    Okta,
    Auth0,
    Keycloak,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SsoAuthRequest {
    pub provider: SsoProvider,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SsoAuthResponse {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: Option<String>, // For PKCE
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SsoCallbackRequest {
    pub code: String,
    pub state: String,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SsoUserInfo {
    pub email: String,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub organization: Option<String>,
    pub groups: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SsoManager {
    configs: HashMap<String, SsoConfig>,
    jwt_manager: Arc<JwtManager>,
    user_service: Arc<dyn UserService>,
}

impl SsoManager {
    pub fn new(
        jwt_manager: Arc<JwtManager>,
        user_service: Arc<dyn UserService>,
    ) -> Self {
        Self {
            configs: HashMap::new(),
            jwt_manager,
            user_service,
        }
    }

    pub fn add_provider(&mut self, organization_id: &str, config: SsoConfig) {
        self.configs.insert(organization_id.to_string(), config);
    }

    pub async fn initiate_auth(&self, organization_id: &str, request: SsoAuthRequest) -> Result<SsoAuthResponse> {
        let config = self.configs.get(organization_id)
            .ok_or_else(|| anyhow::anyhow!("SSO not configured for organization"))?;

        if !config.enabled {
            return Err(anyhow::anyhow!("SSO is disabled for this organization"));
        }

        match config.provider {
            SsoProvider::Oidc | SsoProvider::AzureAd | SsoProvider::GoogleWorkspace | 
            SsoProvider::Auth0 | SsoProvider::Keycloak => {
                self.initiate_oauth2_auth(config, request).await
            }
            SsoProvider::Okta => {
                self.initiate_okta_auth(config, request).await
            }
            SsoProvider::Saml => {
                self.initiate_saml_auth(config, request).await
            }
        }
    }

    async fn initiate_oauth2_auth(&self, config: &SsoConfig, request: SsoAuthRequest) -> Result<SsoAuthResponse> {
        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())?,
            Some(TokenUrl::new(config.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url.clone())?);

        // Generate PKCE challenge for enhanced security
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Build authorization URL
        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        // Add scopes
        for scope in &config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token) = auth_request.url();

        Ok(SsoAuthResponse {
            auth_url: auth_url.to_string(),
            state: csrf_token.secret().clone(),
            code_verifier: Some(pkce_verifier.secret().clone()),
        })
    }

    async fn initiate_okta_auth(&self, config: &SsoConfig, request: SsoAuthRequest) -> Result<SsoAuthResponse> {
        // Okta-specific OAuth2 implementation
        self.initiate_oauth2_auth(config, request).await
    }

    async fn initiate_saml_auth(&self, config: &SsoConfig, request: SsoAuthRequest) -> Result<SsoAuthResponse> {
        // SAML implementation would go here
        // For now, return an error as SAML requires more complex implementation
        Err(anyhow::anyhow!("SAML authentication not yet implemented"))
    }

    pub async fn handle_callback(
        &self,
        organization_id: &str,
        callback: SsoCallbackRequest,
    ) -> Result<TokenPair> {
        let config = self.configs.get(organization_id)
            .ok_or_else(|| anyhow::anyhow!("SSO not configured for organization"))?;

        match config.provider {
            SsoProvider::Oidc | SsoProvider::AzureAd | SsoProvider::GoogleWorkspace | 
            SsoProvider::Auth0 | SsoProvider::Keycloak | SsoProvider::Okta => {
                self.handle_oauth2_callback(config, callback, organization_id).await
            }
            SsoProvider::Saml => {
                self.handle_saml_callback(config, callback, organization_id).await
            }
        }
    }

    async fn handle_oauth2_callback(
        &self,
        config: &SsoConfig,
        callback: SsoCallbackRequest,
        organization_id: &str,
    ) -> Result<TokenPair> {
        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())?,
            Some(TokenUrl::new(config.token_url.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url.clone())?);

        // Exchange authorization code for access token
        let mut token_request = client
            .exchange_code(AuthorizationCode::new(callback.code));

        // Add PKCE verifier if provided
        if let Some(code_verifier) = callback.code_verifier {
            token_request = token_request.set_pkce_verifier(oauth2::PkceCodeVerifier::new(code_verifier));
        }

        let token_result = token_request
            .request_async(async_http_client)
            .await?;

        // Get user info using access token
        let user_info = self.get_user_info(config, token_result.access_token().secret()).await?;

        // Find or create user
        let user = self.find_or_create_user(&user_info, organization_id).await?;

        // Generate session and JWT tokens
        let session_id = Uuid::new_v4();
        let token_pair = self.jwt_manager.generate_token_pair(&user, session_id)?;

        // Update last login
        if let Err(e) = self.user_service.update_last_login(user.id).await {
            warn!("Failed to update last login for user {}: {}", user.email, e);
        }

        info!("SSO authentication successful for user: {}", user.email);

        Ok(token_pair)
    }

    async fn handle_saml_callback(
        &self,
        config: &SsoConfig,
        callback: SsoCallbackRequest,
        organization_id: &str,
    ) -> Result<TokenPair> {
        // SAML callback handling would go here
        Err(anyhow::anyhow!("SAML callback handling not yet implemented"))
    }

    async fn get_user_info(&self, config: &SsoConfig, access_token: &str) -> Result<SsoUserInfo> {
        let client = reqwest::Client::new();
        
        let response = client
            .get(&config.userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get user info: {}", response.status()));
        }

        let user_data: serde_json::Value = response.json().await?;
        
        // Parse user info based on provider
        let user_info = match config.provider {
            SsoProvider::GoogleWorkspace => self.parse_google_user_info(&user_data)?,
            SsoProvider::AzureAd => self.parse_azure_user_info(&user_data)?,
            SsoProvider::Auth0 => self.parse_auth0_user_info(&user_data)?,
            SsoProvider::Okta => self.parse_okta_user_info(&user_data)?,
            SsoProvider::Keycloak => self.parse_keycloak_user_info(&user_data)?,
            SsoProvider::Oidc => self.parse_oidc_user_info(&user_data)?,
            SsoProvider::Saml => return Err(anyhow::anyhow!("SAML user info parsing not implemented")),
        };

        Ok(user_info)
    }

    fn parse_google_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        Ok(SsoUserInfo {
            email: data["email"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str().unwrap_or_default().to_string(),
            given_name: data["given_name"].as_str().map(|s| s.to_string()),
            family_name: data["family_name"].as_str().map(|s| s.to_string()),
            picture: data["picture"].as_str().map(|s| s.to_string()),
            organization: data["hd"].as_str().map(|s| s.to_string()), // Google Workspace domain
            groups: vec![], // Google doesn't provide groups in userinfo
            roles: vec![],
        })
    }

    fn parse_azure_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        Ok(SsoUserInfo {
            email: data["mail"].as_str()
                .or_else(|| data["userPrincipalName"].as_str())
                .unwrap_or_default().to_string(),
            name: data["displayName"].as_str().unwrap_or_default().to_string(),
            given_name: data["givenName"].as_str().map(|s| s.to_string()),
            family_name: data["surname"].as_str().map(|s| s.to_string()),
            picture: None,
            organization: data["companyName"].as_str().map(|s| s.to_string()),
            groups: data["groups"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            roles: data["roles"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    fn parse_auth0_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        Ok(SsoUserInfo {
            email: data["email"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str().unwrap_or_default().to_string(),
            given_name: data["given_name"].as_str().map(|s| s.to_string()),
            family_name: data["family_name"].as_str().map(|s| s.to_string()),
            picture: data["picture"].as_str().map(|s| s.to_string()),
            organization: data["org_id"].as_str().map(|s| s.to_string()),
            groups: data["groups"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            roles: data["roles"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    fn parse_okta_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        Ok(SsoUserInfo {
            email: data["email"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str().unwrap_or_default().to_string(),
            given_name: data["given_name"].as_str().map(|s| s.to_string()),
            family_name: data["family_name"].as_str().map(|s| s.to_string()),
            picture: None,
            organization: data["organization"].as_str().map(|s| s.to_string()),
            groups: data["groups"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            roles: vec![],
        })
    }

    fn parse_keycloak_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        Ok(SsoUserInfo {
            email: data["email"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str().unwrap_or_default().to_string(),
            given_name: data["given_name"].as_str().map(|s| s.to_string()),
            family_name: data["family_name"].as_str().map(|s| s.to_string()),
            picture: None,
            organization: data["organization"].as_str().map(|s| s.to_string()),
            groups: data["groups"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            roles: data["realm_access"]["roles"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    fn parse_oidc_user_info(&self, data: &serde_json::Value) -> Result<SsoUserInfo> {
        // Generic OIDC parsing
        Ok(SsoUserInfo {
            email: data["email"].as_str().unwrap_or_default().to_string(),
            name: data["name"].as_str()
                .or_else(|| data["preferred_username"].as_str())
                .unwrap_or_default().to_string(),
            given_name: data["given_name"].as_str().map(|s| s.to_string()),
            family_name: data["family_name"].as_str().map(|s| s.to_string()),
            picture: data["picture"].as_str().map(|s| s.to_string()),
            organization: data["organization"].as_str().map(|s| s.to_string()),
            groups: data["groups"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            roles: data["roles"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    async fn find_or_create_user(&self, user_info: &SsoUserInfo, organization_id: &str) -> Result<User> {
        // Try to find existing user by email
        if let Some(existing_user) = self.user_service.get_user_by_email(&user_info.email).await? {
            // Update user info from SSO
            return Ok(existing_user);
        }

        // Create new user
        let org_id = Uuid::parse_str(organization_id)?;
        let user_id = Uuid::new_v4();

        // Map SSO roles to system roles
        let roles = self.map_sso_roles_to_system_roles(&user_info.roles, org_id);

        let user = User {
            id: user_id,
            email: user_info.email.clone(),
            name: user_info.name.clone(),
            organization_id: org_id,
            roles,
            permissions: vec![Permission::ApiAccess], // Default permission
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // In a real implementation, this would save to database
        debug!("Created new user from SSO: {}", user.email);

        Ok(user)
    }

    fn map_sso_roles_to_system_roles(&self, sso_roles: &[String], org_id: Uuid) -> Vec<Role> {
        let mut roles = vec![];

        for sso_role in sso_roles {
            match sso_role.to_lowercase().as_str() {
                "admin" | "administrator" | "owner" => {
                    roles.push(Role::admin_role(org_id));
                }
                "developer" | "engineer" | "dev" => {
                    roles.push(Role::developer_role(org_id));
                }
                "viewer" | "read-only" | "guest" => {
                    roles.push(Role::viewer_role(org_id));
                }
                "auditor" | "compliance" => {
                    roles.push(Role::auditor_role(org_id));
                }
                _ => {
                    // Unknown role, assign developer by default
                    debug!("Unknown SSO role '{}', assigning developer role", sso_role);
                }
            }
        }

        // If no roles mapped, assign developer role as default
        if roles.is_empty() {
            roles.push(Role::developer_role(org_id));
        }

        roles
    }

    pub fn get_provider_config(&self, organization_id: &str) -> Option<&SsoConfig> {
        self.configs.get(organization_id)
    }

    pub fn is_sso_enabled(&self, organization_id: &str) -> bool {
        self.configs.get(organization_id)
            .map(|config| config.enabled)
            .unwrap_or(false)
    }
}