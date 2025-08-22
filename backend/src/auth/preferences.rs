use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: Uuid,
    pub default_provider: String,
    pub default_model: String,
    pub max_tokens: i32,
    pub temperature: f64,
    pub auto_save: bool,
    pub create_backups: bool,
    pub theme: String,
    pub language: String,
    pub timezone: String,
    pub notifications: NotificationSettings,
    pub editor_settings: EditorSettings,
    pub ai_settings: AISettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub email: bool,
    pub push: bool,
    pub desktop: bool,
    pub completion_alerts: bool,
    pub cost_alerts: bool,
    pub security_alerts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_size: i32,
    pub tab_size: i32,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub syntax_highlighting: bool,
    pub auto_complete: bool,
    pub vim_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISettings {
    pub enable_inline_completion: bool,
    pub enable_code_explanation: bool,
    pub enable_auto_documentation: bool,
    pub enable_security_scanning: bool,
    pub enable_performance_hints: bool,
    pub preferred_explanation_style: String,
    pub code_review_strictness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub auto_save: Option<bool>,
    pub create_backups: Option<bool>,
    pub theme: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub notifications: Option<NotificationSettings>,
    pub editor_settings: Option<EditorSettings>,
    pub ai_settings: Option<AISettings>,
}

pub struct PreferencesService {
    pool: PgPool,
}

impl PreferencesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user_preferences(&self, user_id: Uuid) -> Result<UserPreferences> {
        let row = sqlx::query!(
            r#"
            SELECT user_id, default_provider, default_model, max_tokens, temperature,
                   auto_save, create_backups, theme, language, timezone,
                   notifications, editor_settings, ai_settings
            FROM user_preferences
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(UserPreferences {
                user_id: row.user_id,
                default_provider: row.default_provider.unwrap_or_else(|| "openrouter".to_string()),
                default_model: row.default_model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
                max_tokens: row.max_tokens.unwrap_or(4000),
                temperature: row.temperature.unwrap_or(0.7),
                auto_save: row.auto_save.unwrap_or(true),
                create_backups: row.create_backups.unwrap_or(true),
                theme: row.theme.unwrap_or_else(|| "dark".to_string()),
                language: row.language.unwrap_or_else(|| "en".to_string()),
                timezone: row.timezone.unwrap_or_else(|| "UTC".to_string()),
                notifications: serde_json::from_value(row.notifications.unwrap_or_else(|| {
                    serde_json::json!({
                        "email": true,
                        "push": true,
                        "desktop": false,
                        "completion_alerts": true,
                        "cost_alerts": true,
                        "security_alerts": true
                    })
                }))?,
                editor_settings: serde_json::from_value(row.editor_settings.unwrap_or_else(|| {
                    serde_json::json!({
                        "font_size": 14,
                        "tab_size": 2,
                        "word_wrap": true,
                        "line_numbers": true,
                        "syntax_highlighting": true,
                        "auto_complete": true,
                        "vim_mode": false
                    })
                }))?,
                ai_settings: serde_json::from_value(row.ai_settings.unwrap_or_else(|| {
                    serde_json::json!({
                        "enable_inline_completion": true,
                        "enable_code_explanation": true,
                        "enable_auto_documentation": false,
                        "enable_security_scanning": true,
                        "enable_performance_hints": true,
                        "preferred_explanation_style": "detailed",
                        "code_review_strictness": "medium"
                    })
                }))?,
            })
        } else {
            // Create default preferences
            self.create_default_preferences(user_id).await
        }
    }

    pub async fn update_user_preferences(
        &self,
        user_id: Uuid,
        request: UpdatePreferencesRequest,
    ) -> Result<UserPreferences> {
        // First ensure preferences exist
        let _existing = self.get_user_preferences(user_id).await?;

        // Update preferences
        sqlx::query!(
            r#"
            UPDATE user_preferences
            SET 
                default_provider = COALESCE($2, default_provider),
                default_model = COALESCE($3, default_model),
                max_tokens = COALESCE($4, max_tokens),
                temperature = COALESCE($5, temperature),
                auto_save = COALESCE($6, auto_save),
                create_backups = COALESCE($7, create_backups),
                theme = COALESCE($8, theme),
                language = COALESCE($9, language),
                timezone = COALESCE($10, timezone),
                notifications = COALESCE($11, notifications),
                editor_settings = COALESCE($12, editor_settings),
                ai_settings = COALESCE($13, ai_settings),
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id,
            request.default_provider,
            request.default_model,
            request.max_tokens,
            request.temperature,
            request.auto_save,
            request.create_backups,
            request.theme,
            request.language,
            request.timezone,
            request.notifications.map(|n| serde_json::to_value(n)).transpose()?,
            request.editor_settings.map(|e| serde_json::to_value(e)).transpose()?,
            request.ai_settings.map(|a| serde_json::to_value(a)).transpose()?,
        )
        .execute(&self.pool)
        .await?;

        // Return updated preferences
        self.get_user_preferences(user_id).await
    }

    async fn create_default_preferences(&self, user_id: Uuid) -> Result<UserPreferences> {
        let default_notifications = NotificationSettings {
            email: true,
            push: true,
            desktop: false,
            completion_alerts: true,
            cost_alerts: true,
            security_alerts: true,
        };

        let default_editor = EditorSettings {
            font_size: 14,
            tab_size: 2,
            word_wrap: true,
            line_numbers: true,
            syntax_highlighting: true,
            auto_complete: true,
            vim_mode: false,
        };

        let default_ai = AISettings {
            enable_inline_completion: true,
            enable_code_explanation: true,
            enable_auto_documentation: false,
            enable_security_scanning: true,
            enable_performance_hints: true,
            preferred_explanation_style: "detailed".to_string(),
            code_review_strictness: "medium".to_string(),
        };

        sqlx::query!(
            r#"
            INSERT INTO user_preferences (
                user_id, default_provider, default_model, max_tokens, temperature,
                auto_save, create_backups, theme, language, timezone,
                notifications, editor_settings, ai_settings
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id,
            "openrouter",
            "gpt-4o-mini",
            4000,
            0.7,
            true,
            true,
            "dark",
            "en",
            "UTC",
            serde_json::to_value(&default_notifications)?,
            serde_json::to_value(&default_editor)?,
            serde_json::to_value(&default_ai)?,
        )
        .execute(&self.pool)
        .await?;

        Ok(UserPreferences {
            user_id,
            default_provider: "openrouter".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            max_tokens: 4000,
            temperature: 0.7,
            auto_save: true,
            create_backups: true,
            theme: "dark".to_string(),
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            notifications: default_notifications,
            editor_settings: default_editor,
            ai_settings: default_ai,
        })
    }

    pub async fn reset_preferences(&self, user_id: Uuid) -> Result<UserPreferences> {
        sqlx::query!(
            "DELETE FROM user_preferences WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        self.create_default_preferences(user_id).await
    }

    pub async fn export_preferences(&self, user_id: Uuid) -> Result<serde_json::Value> {
        let preferences = self.get_user_preferences(user_id).await?;
        Ok(serde_json::to_value(preferences)?)
    }

    pub async fn import_preferences(
        &self,
        user_id: Uuid,
        preferences_json: serde_json::Value,
    ) -> Result<UserPreferences> {
        let preferences: UserPreferences = serde_json::from_value(preferences_json)?;
        
        let request = UpdatePreferencesRequest {
            default_provider: Some(preferences.default_provider),
            default_model: Some(preferences.default_model),
            max_tokens: Some(preferences.max_tokens),
            temperature: Some(preferences.temperature),
            auto_save: Some(preferences.auto_save),
            create_backups: Some(preferences.create_backups),
            theme: Some(preferences.theme),
            language: Some(preferences.language),
            timezone: Some(preferences.timezone),
            notifications: Some(preferences.notifications),
            editor_settings: Some(preferences.editor_settings),
            ai_settings: Some(preferences.ai_settings),
        };

        self.update_user_preferences(user_id, request).await
    }
}