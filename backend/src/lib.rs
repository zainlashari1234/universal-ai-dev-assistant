// Universal AI Development Assistant - Library Root

pub mod config;
pub mod models;
pub mod auth;
pub mod api;
pub mod providers;
pub mod search;
pub mod sandbox;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use models::*;

// Library initialization
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Initialize other components
    Ok(())
}

// Health check function for testing
pub fn health_check() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        assert!(health_check());
    }

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}