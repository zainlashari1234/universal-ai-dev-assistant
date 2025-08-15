// P0 Day-3: Database connection and management module
use anyhow::Result;
use sqlx::{PgPool, Pool, Postgres, migrate::MigrateDatabase};
use std::env;
use tracing::{info, warn, error};

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    /// Initialize database connection and run migrations
    pub async fn new() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://uaida:uaida123@localhost:5432/uaida_dev".to_string());
        
        info!("Connecting to database: {}", mask_password(&database_url));
        
        // Create database if it doesn't exist
        if !Postgres::database_exists(&database_url).await.unwrap_or(false) {
            info!("Database does not exist, creating...");
            Postgres::create_database(&database_url).await?;
            info!("Database created successfully");
        }
        
        // Create connection pool
        let pool = PgPool::connect(&database_url).await?;
        
        info!("Database connection established");
        
        let manager = DatabaseManager { pool };
        
        // Run migrations
        manager.run_migrations().await?;
        
        Ok(manager)
    }
    
    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        
        info!("Database migrations completed successfully");
        Ok(())
    }
    
    /// Check database health
    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let start_time = std::time::Instant::now();
        
        // Simple query to test connection
        let result = sqlx::query_scalar::<_, i64>("SELECT 1")
            .fetch_one(&self.pool)
            .await;
        
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => Ok(DatabaseHealth {
                connected: true,
                latency_ms: Some(latency_ms),
                pool_size: self.pool.size(),
                active_connections: self.pool.size() - self.pool.num_idle(),
                error: None,
            }),
            Err(e) => Ok(DatabaseHealth {
                connected: false,
                latency_ms: None,
                pool_size: self.pool.size(),
                active_connections: 0,
                error: Some(e.to_string()),
            }),
        }
    }
    
    /// Close database connections
    pub async fn close(&self) {
        info!("Closing database connections");
        self.pool.close().await;
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
            
        let project_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await?;
            
        let run_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runs")
            .fetch_one(&self.pool)
            .await?;
            
        let artifact_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM artifacts")
            .fetch_one(&self.pool)
            .await?;
            
        let completion_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM completion_logs")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(DatabaseStats {
            users: user_count,
            projects: project_count,
            runs: run_count,
            artifacts: artifact_count,
            completions: completion_count,
        })
    }
}

/// Database health status
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub latency_ms: Option<u64>,
    pub pool_size: u32,
    pub active_connections: u32,
    pub error: Option<String>,
}

/// Database statistics
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    pub users: i64,
    pub projects: i64,
    pub runs: i64,
    pub artifacts: i64,
    pub completions: i64,
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[0..at_pos].rfind(':') {
            let mut masked = url.to_string();
            let password_start = colon_pos + 1;
            let password_end = at_pos;
            
            if password_end > password_start {
                masked.replace_range(password_start..password_end, "***");
            }
            
            return masked;
        }
    }
    url.to_string()
}