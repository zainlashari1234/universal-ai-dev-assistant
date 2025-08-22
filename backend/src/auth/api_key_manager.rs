use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub key_name: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: i32,
    pub monthly_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub provider: String,
    pub key_name: String,
    pub api_key: String,
    pub monthly_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub provider: String,
    pub key_name: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: i32,
    pub monthly_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
}

pub struct ApiKeyManager {
    pool: PgPool,
    encryption_key: [u8; 32],
}

impl ApiKeyManager {
    pub fn new(pool: PgPool, encryption_key: [u8; 32]) -> Self {
        Self {
            pool,
            encryption_key,
        }
    }

    /// Encrypt API key using AES-256-GCM
    pub fn encrypt_key(&self, key: &str) -> Result<String> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        let ciphertext = cipher
            .encrypt(&nonce, key.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Combine nonce and ciphertext
        let mut encrypted_data = nonce.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);
        
        Ok(general_purpose::STANDARD.encode(&encrypted_data))
    }

    /// Decrypt API key using AES-256-GCM
    pub fn decrypt_key(&self, encrypted: &str) -> Result<String> {
        let encrypted_data = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;
        
        if encrypted_data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data length"));
        }
        
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| anyhow!("UTF-8 conversion failed: {}", e))
    }

    /// Generate hash for quick key validation
    fn generate_key_hash(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(&self.encryption_key);
        format!("{:x}", hasher.finalize())
    }

    /// Store encrypted API key
    pub async fn store_api_key(&self, user_id: Uuid, request: CreateApiKeyRequest) -> Result<ApiKey> {
        let encrypted_key = self.encrypt_key(&request.api_key)?;
        let key_hash = self.generate_key_hash(&request.api_key);
        
        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            INSERT INTO api_keys (user_id, provider, key_name, encrypted_key, key_hash, monthly_limit)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, provider, key_name, is_active, last_used_at, usage_count, monthly_limit, created_at, updated_at
            "#,
            user_id,
            request.provider,
            request.key_name,
            encrypted_key,
            key_hash,
            request.monthly_limit
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(api_key)
    }

    /// Get user's API keys (without decrypted values)
    pub async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKeyResponse>> {
        let keys = sqlx::query_as!(
            ApiKeyResponse,
            r#"
            SELECT id, provider, key_name, is_active, last_used_at, usage_count, monthly_limit, created_at
            FROM api_keys
            WHERE user_id = $1 AND is_active = true
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Get decrypted API key for provider
    pub async fn get_api_key(&self, user_id: Uuid, provider: &str) -> Result<Option<String>> {
        let row = sqlx::query!(
            r#"
            SELECT encrypted_key
            FROM api_keys
            WHERE user_id = $1 AND provider = $2 AND is_active = true
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id,
            provider
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let decrypted = self.decrypt_key(&row.encrypted_key)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    /// Validate API key and check limits
    pub async fn validate_key(&self, user_id: Uuid, provider: &str, key: &str) -> Result<bool> {
        let key_hash = self.generate_key_hash(key);
        
        let row = sqlx::query!(
            r#"
            SELECT id, monthly_limit
            FROM api_keys
            WHERE user_id = $1 AND provider = $2 AND key_hash = $3 AND is_active = true
            "#,
            user_id,
            provider,
            key_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // Check monthly limit if set
            if let Some(limit) = row.monthly_limit {
                let usage_this_month = sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*)
                    FROM completion_logs cl
                    WHERE cl.user_id = $1
                    AND DATE_TRUNC('month', cl.created_at) = DATE_TRUNC('month', NOW())
                    "#,
                    user_id
                )
                .fetch_one(&self.pool)
                .await?;

                if usage_this_month.unwrap_or(0) >= limit as i64 {
                    return Ok(false);
                }
            }

            // Update usage
            sqlx::query!(
                "SELECT update_api_key_usage($1)",
                key_hash
            )
            .execute(&self.pool)
            .await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete API key
    pub async fn delete_api_key(&self, user_id: Uuid, key_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE api_keys
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            "#,
            key_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get API key usage statistics
    pub async fn get_usage_stats(&self, user_id: Uuid) -> Result<HashMap<String, i64>> {
        let rows = sqlx::query!(
            r#"
            SELECT ak.provider, COUNT(cl.id) as usage_count
            FROM api_keys ak
            LEFT JOIN completion_logs cl ON cl.user_id = ak.user_id
            WHERE ak.user_id = $1 AND ak.is_active = true
            AND DATE_TRUNC('month', cl.created_at) = DATE_TRUNC('month', NOW())
            GROUP BY ak.provider
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut stats = HashMap::new();
        for row in rows {
            stats.insert(row.provider, row.usage_count.unwrap_or(0));
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = [0u8; 32];
        let manager = ApiKeyManager::new(
            // Mock pool - not used in this test
            unsafe { std::mem::zeroed() },
            key
        );

        let original = "sk-test-key-12345";
        let encrypted = manager.encrypt_key(original).unwrap();
        let decrypted = manager.decrypt_key(&encrypted).unwrap();

        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_key_hash_generation() {
        let key = [0u8; 32];
        let manager = ApiKeyManager::new(
            unsafe { std::mem::zeroed() },
            key
        );

        let api_key = "sk-test-key-12345";
        let hash1 = manager.generate_key_hash(api_key);
        let hash2 = manager.generate_key_hash(api_key);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string
    }
}