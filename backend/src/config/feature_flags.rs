// P0 Day-5: Feature flags system for experimental toggles
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    // Experimental features
    pub experimental_risk_gate: bool,
    pub experimental_advanced_tracing: bool,
    pub experimental_ai_code_review: bool,
    pub experimental_auto_fix: bool,
    pub experimental_multi_provider: bool,
    
    // Production features
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_security_headers: bool,
    pub enable_rate_limiting: bool,
    pub enable_cors: bool,
    pub enable_database_persistence: bool,
    
    // Evaluation features
    pub enable_evaluations: bool,
    pub auto_run_evals: bool,
    
    // Feature toggles for A/B testing
    pub toggles: HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        let mut toggles = HashMap::new();
        toggles.insert("use_new_completion_algorithm".to_string(), false);
        toggles.insert("enable_collaborative_editing".to_string(), false);
        toggles.insert("enable_voice_commands".to_string(), false);
        toggles.insert("enable_mobile_support".to_string(), false);
        
        Self {
            experimental_risk_gate: true,
            experimental_advanced_tracing: true,
            experimental_ai_code_review: false,
            experimental_auto_fix: false,
            experimental_multi_provider: true,
            enable_metrics: true,
            enable_tracing: true,
            enable_security_headers: true,
            enable_rate_limiting: true,
            enable_cors: true,
            enable_database_persistence: true,
            enable_evaluations: true,
            auto_run_evals: false,
            toggles,
        }
    }
}

impl FeatureFlags {
    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature_name: &str) -> bool {
        match feature_name {
            "experimental_risk_gate" => self.experimental_risk_gate,
            "experimental_advanced_tracing" => self.experimental_advanced_tracing,
            "experimental_ai_code_review" => self.experimental_ai_code_review,
            "experimental_auto_fix" => self.experimental_auto_fix,
            "experimental_multi_provider" => self.experimental_multi_provider,
            "enable_metrics" => self.enable_metrics,
            "enable_tracing" => self.enable_tracing,
            "enable_security_headers" => self.enable_security_headers,
            "enable_rate_limiting" => self.enable_rate_limiting,
            "enable_cors" => self.enable_cors,
            "enable_database_persistence" => self.enable_database_persistence,
            "enable_evaluations" => self.enable_evaluations,
            "auto_run_evals" => self.auto_run_evals,
            _ => {
                // Check in toggles
                self.toggles.get(feature_name).copied().unwrap_or(false)
            }
        }
    }
    
    /// Enable a feature dynamically
    pub fn enable_feature(&mut self, feature_name: &str) {
        info!(feature = feature_name, "Enabling feature");
        
        match feature_name {
            "experimental_risk_gate" => self.experimental_risk_gate = true,
            "experimental_advanced_tracing" => self.experimental_advanced_tracing = true,
            "experimental_ai_code_review" => self.experimental_ai_code_review = true,
            "experimental_auto_fix" => self.experimental_auto_fix = true,
            "experimental_multi_provider" => self.experimental_multi_provider = true,
            "enable_evaluations" => self.enable_evaluations = true,
            "auto_run_evals" => self.auto_run_evals = true,
            _ => {
                self.toggles.insert(feature_name.to_string(), true);
            }
        }
    }
    
    /// Disable a feature dynamically
    pub fn disable_feature(&mut self, feature_name: &str) {
        info!(feature = feature_name, "Disabling feature");
        
        match feature_name {
            "experimental_risk_gate" => self.experimental_risk_gate = false,
            "experimental_advanced_tracing" => self.experimental_advanced_tracing = false,
            "experimental_ai_code_review" => self.experimental_ai_code_review = false,
            "experimental_auto_fix" => self.experimental_auto_fix = false,
            "experimental_multi_provider" => self.experimental_multi_provider = false,
            "enable_evaluations" => self.enable_evaluations = false,
            "auto_run_evals" => self.auto_run_evals = false,
            _ => {
                self.toggles.insert(feature_name.to_string(), false);
            }
        }
    }
    
    /// Get all experimental features that are enabled
    pub fn get_enabled_experimental_features(&self) -> Vec<String> {
        let mut enabled = Vec::new();
        
        if self.experimental_risk_gate {
            enabled.push("experimental_risk_gate".to_string());
        }
        if self.experimental_advanced_tracing {
            enabled.push("experimental_advanced_tracing".to_string());
        }
        if self.experimental_ai_code_review {
            enabled.push("experimental_ai_code_review".to_string());
        }
        if self.experimental_auto_fix {
            enabled.push("experimental_auto_fix".to_string());
        }
        if self.experimental_multi_provider {
            enabled.push("experimental_multi_provider".to_string());
        }
        
        enabled
    }
    
    /// Validate feature flag configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check for conflicting settings
        if !self.enable_database_persistence && self.enable_evaluations {
            warn!("Evaluations enabled but database persistence disabled - some features may not work");
        }
        
        if !self.enable_tracing && self.experimental_advanced_tracing {
            return Err("Advanced tracing requires basic tracing to be enabled".to_string());
        }
        
        if !self.enable_metrics && self.experimental_risk_gate {
            warn!("Risk gate enabled but metrics disabled - performance monitoring limited");
        }
        
        // Check experimental feature dependencies
        if self.experimental_ai_code_review && !self.experimental_multi_provider {
            warn!("AI code review works best with multi-provider support enabled");
        }
        
        info!("Feature flags validation completed successfully");
        Ok(())
    }
    
    /// Get feature flag summary for logging
    pub fn get_summary(&self) -> String {
        let experimental_count = self.get_enabled_experimental_features().len();
        let production_features = vec![
            ("metrics", self.enable_metrics),
            ("tracing", self.enable_tracing),
            ("security", self.enable_security_headers),
            ("database", self.enable_database_persistence),
            ("evaluations", self.enable_evaluations),
        ];
        let production_enabled = production_features.iter().filter(|(_, enabled)| *enabled).count();
        
        format!(
            "Features: {} experimental, {}/{} production, {} toggles",
            experimental_count,
            production_enabled,
            production_features.len(),
            self.toggles.len()
        )
    }
}