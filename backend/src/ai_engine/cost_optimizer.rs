use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CostOptimizer {
    provider_metrics: RwLock<HashMap<String, ProviderMetrics>>,
    cost_tracker: RwLock<CostTracker>,
    optimization_config: OptimizationConfig,
    usage_analytics: RwLock<UsageAnalytics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub cost_weight: f32,
    pub performance_weight: f32,
    pub reliability_weight: f32,
    pub max_cost_per_request: f32,
    pub daily_budget_limit: f32,
    pub enable_auto_switching: bool,
    pub fallback_strategy: FallbackStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackStrategy {
    CheapestFirst,
    FastestFirst,
    MostReliable,
    Balanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub provider_name: String,
    pub cost_per_token: f32,
    pub average_latency_ms: f32,
    pub success_rate: f32,
    pub tokens_processed_today: u64,
    pub cost_spent_today: f32,
    pub last_updated: u64,
    pub performance_score: f32,
    pub reliability_score: f32,
    pub cost_efficiency_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTracker {
    pub daily_costs: HashMap<String, DailyCost>, // provider -> daily cost
    pub monthly_costs: HashMap<String, MonthlyCost>,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f32,
    pub cost_savings: f32,
    pub optimization_decisions: Vec<OptimizationDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub provider: String,
    pub requests: u64,
    pub tokens: u64,
    pub cost: f32,
    pub average_cost_per_request: f32,
    pub average_cost_per_token: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyCost {
    pub month: String,
    pub provider: String,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f32,
    pub daily_breakdown: Vec<DailyCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationDecision {
    pub timestamp: u64,
    pub request_id: String,
    pub original_provider: String,
    pub selected_provider: String,
    pub reason: String,
    pub cost_saved: f32,
    pub performance_impact: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub hourly_usage: HashMap<u8, HourlyUsage>, // hour -> usage
    pub provider_usage_patterns: HashMap<String, UsagePattern>,
    pub cost_trends: Vec<CostTrend>,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyUsage {
    pub hour: u8,
    pub requests: u64,
    pub tokens: u64,
    pub cost: f32,
    pub average_latency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub provider: String,
    pub peak_hours: Vec<u8>,
    pub average_request_size: f32,
    pub preferred_use_cases: Vec<String>,
    pub cost_efficiency_by_hour: HashMap<u8, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrend {
    pub period: String,
    pub provider: String,
    pub cost_change_percentage: f32,
    pub volume_change_percentage: f32,
    pub efficiency_change_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOpportunity {
    pub opportunity_type: OpportunityType,
    pub description: String,
    pub potential_savings: f32,
    pub implementation_effort: EffortLevel,
    pub impact_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpportunityType {
    ProviderSwitch,
    RequestBatching,
    CacheOptimization,
    ModelDowngrade,
    SchedulingOptimization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSelection {
    pub selected_provider: String,
    pub selection_reason: String,
    pub estimated_cost: f32,
    pub estimated_latency: f32,
    pub confidence_score: f32,
    pub alternatives: Vec<ProviderAlternative>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAlternative {
    pub provider: String,
    pub cost: f32,
    pub latency: f32,
    pub reliability: f32,
    pub overall_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationRequest {
    pub request_type: String,
    pub estimated_tokens: u32,
    pub priority: RequestPriority,
    pub max_latency_ms: Option<u32>,
    pub max_cost: Option<f32>,
    pub required_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl CostOptimizer {
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            provider_metrics: RwLock::new(HashMap::new()),
            cost_tracker: RwLock::new(CostTracker::new()),
            optimization_config: config,
            usage_analytics: RwLock::new(UsageAnalytics::new()),
        }
    }

    pub async fn select_optimal_provider(
        &self,
        request: &CostOptimizationRequest,
        available_providers: &[String],
    ) -> Result<ProviderSelection> {
        let metrics = self.provider_metrics.read().await;
        let mut scored_providers = Vec::new();

        for provider in available_providers {
            if let Some(provider_metrics) = metrics.get(provider) {
                let score = self.calculate_provider_score(provider_metrics, request).await?;
                scored_providers.push((provider.clone(), score, provider_metrics.clone()));
            }
        }

        // Sort by score (highest first)
        scored_providers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if scored_providers.is_empty() {
            return Err(anyhow::anyhow!("No suitable providers available"));
        }

        let (selected_provider, best_score, best_metrics) = scored_providers[0].clone();
        
        // Create alternatives list
        let alternatives: Vec<ProviderAlternative> = scored_providers
            .iter()
            .skip(1)
            .take(3)
            .map(|(provider, score, metrics)| ProviderAlternative {
                provider: provider.clone(),
                cost: self.estimate_cost(metrics, request.estimated_tokens),
                latency: metrics.average_latency_ms,
                reliability: metrics.reliability_score,
                overall_score: *score,
            })
            .collect();

        let selection = ProviderSelection {
            selected_provider: selected_provider.clone(),
            selection_reason: self.get_selection_reason(&best_metrics, request),
            estimated_cost: self.estimate_cost(&best_metrics, request.estimated_tokens),
            estimated_latency: best_metrics.average_latency_ms,
            confidence_score: best_score,
            alternatives,
        };

        // Record the decision
        self.record_optimization_decision(&selection, request).await?;

        Ok(selection)
    }

    async fn calculate_provider_score(
        &self,
        metrics: &ProviderMetrics,
        request: &CostOptimizationRequest,
    ) -> Result<f32> {
        let estimated_cost = self.estimate_cost(metrics, request.estimated_tokens);
        
        // Check budget constraints
        if let Some(max_cost) = request.max_cost {
            if estimated_cost > max_cost {
                return Ok(0.0); // Exclude if over budget
            }
        }

        // Check latency constraints
        if let Some(max_latency) = request.max_latency_ms {
            if metrics.average_latency_ms > max_latency as f32 {
                return Ok(0.0); // Exclude if too slow
            }
        }

        // Check daily budget
        let cost_tracker = self.cost_tracker.read().await;
        if let Some(daily_cost) = cost_tracker.daily_costs.get(&metrics.provider_name) {
            if daily_cost.cost + estimated_cost > self.optimization_config.daily_budget_limit {
                return Ok(0.0); // Exclude if would exceed daily budget
            }
        }

        // Calculate weighted score
        let cost_score = self.calculate_cost_score(metrics, estimated_cost);
        let performance_score = self.calculate_performance_score(metrics, request);
        let reliability_score = metrics.reliability_score;

        let weighted_score = 
            cost_score * self.optimization_config.cost_weight +
            performance_score * self.optimization_config.performance_weight +
            reliability_score * self.optimization_config.reliability_weight;

        Ok(weighted_score)
    }

    fn calculate_cost_score(&self, metrics: &ProviderMetrics, estimated_cost: f32) -> f32 {
        // Lower cost = higher score
        let max_reasonable_cost = self.optimization_config.max_cost_per_request;
        if estimated_cost >= max_reasonable_cost {
            return 0.0;
        }
        
        1.0 - (estimated_cost / max_reasonable_cost)
    }

    fn calculate_performance_score(&self, metrics: &ProviderMetrics, request: &CostOptimizationRequest) -> f32 {
        let mut score = metrics.performance_score;
        
        // Adjust based on request priority
        match request.priority {
            RequestPriority::Critical => score * 1.2,
            RequestPriority::High => score * 1.1,
            RequestPriority::Medium => score,
            RequestPriority::Low => score * 0.9,
        }
        .min(1.0)
    }

    fn estimate_cost(&self, metrics: &ProviderMetrics, tokens: u32) -> f32 {
        metrics.cost_per_token * tokens as f32
    }

    fn get_selection_reason(&self, metrics: &ProviderMetrics, request: &CostOptimizationRequest) -> String {
        let cost = self.estimate_cost(metrics, request.estimated_tokens);
        
        format!(
            "Selected {} for optimal balance: ${:.4} cost, {:.0}ms latency, {:.1}% reliability",
            metrics.provider_name,
            cost,
            metrics.average_latency_ms,
            metrics.reliability_score * 100.0
        )
    }

    async fn record_optimization_decision(
        &self,
        selection: &ProviderSelection,
        request: &CostOptimizationRequest,
    ) -> Result<()> {
        let mut cost_tracker = self.cost_tracker.write().await;
        
        let decision = OptimizationDecision {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            request_id: Uuid::new_v4().to_string(),
            original_provider: "auto".to_string(), // Would be determined by previous logic
            selected_provider: selection.selected_provider.clone(),
            reason: selection.selection_reason.clone(),
            cost_saved: 0.0, // Would be calculated based on alternatives
            performance_impact: 0.0, // Would be calculated based on alternatives
        };

        cost_tracker.optimization_decisions.push(decision);
        
        Ok(())
    }

    pub async fn update_provider_metrics(
        &self,
        provider: &str,
        request_tokens: u32,
        actual_cost: f32,
        latency_ms: f32,
        success: bool,
    ) -> Result<()> {
        let mut metrics = self.provider_metrics.write().await;
        let mut cost_tracker = self.cost_tracker.write().await;
        
        let provider_metrics = metrics.entry(provider.to_string()).or_insert_with(|| {
            ProviderMetrics {
                provider_name: provider.to_string(),
                cost_per_token: 0.0,
                average_latency_ms: 0.0,
                success_rate: 1.0,
                tokens_processed_today: 0,
                cost_spent_today: 0.0,
                last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                performance_score: 0.5,
                reliability_score: 0.5,
                cost_efficiency_score: 0.5,
            }
        });

        // Update metrics with exponential moving average
        let alpha = 0.1; // Smoothing factor
        
        provider_metrics.cost_per_token = 
            alpha * (actual_cost / request_tokens as f32) + 
            (1.0 - alpha) * provider_metrics.cost_per_token;
        
        provider_metrics.average_latency_ms = 
            alpha * latency_ms + 
            (1.0 - alpha) * provider_metrics.average_latency_ms;
        
        provider_metrics.success_rate = 
            alpha * (if success { 1.0 } else { 0.0 }) + 
            (1.0 - alpha) * provider_metrics.success_rate;

        provider_metrics.tokens_processed_today += request_tokens as u64;
        provider_metrics.cost_spent_today += actual_cost;
        provider_metrics.last_updated = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Recalculate derived scores
        provider_metrics.performance_score = self.calculate_performance_score_from_metrics(provider_metrics);
        provider_metrics.reliability_score = provider_metrics.success_rate;
        provider_metrics.cost_efficiency_score = self.calculate_cost_efficiency_score(provider_metrics);

        // Update cost tracking
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let daily_cost = cost_tracker.daily_costs.entry(provider.to_string()).or_insert_with(|| {
            DailyCost {
                date: today.clone(),
                provider: provider.to_string(),
                requests: 0,
                tokens: 0,
                cost: 0.0,
                average_cost_per_request: 0.0,
                average_cost_per_token: 0.0,
            }
        });

        daily_cost.requests += 1;
        daily_cost.tokens += request_tokens as u64;
        daily_cost.cost += actual_cost;
        daily_cost.average_cost_per_request = daily_cost.cost / daily_cost.requests as f32;
        daily_cost.average_cost_per_token = daily_cost.cost / daily_cost.tokens as f32;

        cost_tracker.total_requests += 1;
        cost_tracker.total_tokens += request_tokens as u64;
        cost_tracker.total_cost += actual_cost;

        Ok(())
    }

    fn calculate_performance_score_from_metrics(&self, metrics: &ProviderMetrics) -> f32 {
        // Lower latency = higher score
        let max_acceptable_latency = 5000.0; // 5 seconds
        let latency_score = (max_acceptable_latency - metrics.average_latency_ms) / max_acceptable_latency;
        latency_score.max(0.0).min(1.0)
    }

    fn calculate_cost_efficiency_score(&self, metrics: &ProviderMetrics) -> f32 {
        // Lower cost per token = higher score
        let max_acceptable_cost_per_token = 0.01; // $0.01 per token
        let cost_score = (max_acceptable_cost_per_token - metrics.cost_per_token) / max_acceptable_cost_per_token;
        cost_score.max(0.0).min(1.0)
    }

    pub async fn get_cost_analytics(&self) -> Result<CostAnalyticsReport> {
        let cost_tracker = self.cost_tracker.read().await;
        let metrics = self.provider_metrics.read().await;
        let usage_analytics = self.usage_analytics.read().await;

        let total_cost = cost_tracker.total_cost;
        let total_requests = cost_tracker.total_requests;
        let average_cost_per_request = if total_requests > 0 {
            total_cost / total_requests as f32
        } else {
            0.0
        };

        // Calculate cost savings from optimization
        let estimated_cost_without_optimization = self.estimate_cost_without_optimization(&cost_tracker).await;
        let cost_savings = estimated_cost_without_optimization - total_cost;
        let savings_percentage = if estimated_cost_without_optimization > 0.0 {
            (cost_savings / estimated_cost_without_optimization) * 100.0
        } else {
            0.0
        };

        // Provider breakdown
        let provider_breakdown: Vec<ProviderCostBreakdown> = metrics
            .iter()
            .map(|(provider, metrics)| ProviderCostBreakdown {
                provider: provider.clone(),
                total_cost: metrics.cost_spent_today,
                total_requests: cost_tracker.daily_costs.get(provider)
                    .map(|d| d.requests)
                    .unwrap_or(0),
                average_cost_per_request: metrics.cost_spent_today / 
                    cost_tracker.daily_costs.get(provider)
                        .map(|d| d.requests as f32)
                        .unwrap_or(1.0),
                cost_per_token: metrics.cost_per_token,
                efficiency_score: metrics.cost_efficiency_score,
            })
            .collect();

        Ok(CostAnalyticsReport {
            total_cost,
            total_requests,
            average_cost_per_request,
            cost_savings,
            savings_percentage,
            provider_breakdown,
            optimization_opportunities: usage_analytics.optimization_opportunities.clone(),
            cost_trends: usage_analytics.cost_trends.clone(),
        })
    }

    async fn estimate_cost_without_optimization(&self, cost_tracker: &CostTracker) -> f32 {
        // This would estimate what the cost would have been without optimization
        // For now, assume 20% higher cost without optimization
        cost_tracker.total_cost * 1.2
    }

    pub async fn generate_optimization_recommendations(&self) -> Result<Vec<OptimizationRecommendation>> {
        let metrics = self.provider_metrics.read().await;
        let cost_tracker = self.cost_tracker.read().await;
        let mut recommendations = Vec::new();

        // Analyze provider usage patterns
        for (provider, provider_metrics) in metrics.iter() {
            // Check if provider is underperforming
            if provider_metrics.cost_efficiency_score < 0.5 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: RecommendationType::ProviderOptimization,
                    title: format!("Consider reducing usage of {}", provider),
                    description: format!(
                        "{} has low cost efficiency ({}). Consider switching to more cost-effective alternatives.",
                        provider,
                        provider_metrics.cost_efficiency_score
                    ),
                    potential_savings: provider_metrics.cost_spent_today * 0.3,
                    implementation_effort: EffortLevel::Low,
                    priority: if provider_metrics.cost_efficiency_score < 0.3 {
                        RecommendationPriority::High
                    } else {
                        RecommendationPriority::Medium
                    },
                });
            }

            // Check for high latency
            if provider_metrics.average_latency_ms > 2000.0 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: RecommendationType::PerformanceOptimization,
                    title: format!("Optimize {} for better performance", provider),
                    description: format!(
                        "{} has high average latency ({:.0}ms). Consider using for non-time-critical requests only.",
                        provider,
                        provider_metrics.average_latency_ms
                    ),
                    potential_savings: 0.0,
                    implementation_effort: EffortLevel::Medium,
                    priority: RecommendationPriority::Medium,
                });
            }
        }

        // Check overall budget utilization
        let daily_total: f32 = cost_tracker.daily_costs.values().map(|d| d.cost).sum();
        if daily_total > self.optimization_config.daily_budget_limit * 0.8 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::BudgetOptimization,
                title: "Daily budget utilization is high".to_string(),
                description: format!(
                    "Current daily spending (${:.2}) is approaching the limit (${:.2}). Consider implementing stricter cost controls.",
                    daily_total,
                    self.optimization_config.daily_budget_limit
                ),
                potential_savings: daily_total * 0.2,
                implementation_effort: EffortLevel::High,
                priority: RecommendationPriority::High,
            });
        }

        Ok(recommendations)
    }
}

impl CostTracker {
    fn new() -> Self {
        Self {
            daily_costs: HashMap::new(),
            monthly_costs: HashMap::new(),
            total_requests: 0,
            total_tokens: 0,
            total_cost: 0.0,
            cost_savings: 0.0,
            optimization_decisions: Vec::new(),
        }
    }
}

impl UsageAnalytics {
    fn new() -> Self {
        Self {
            hourly_usage: HashMap::new(),
            provider_usage_patterns: HashMap::new(),
            cost_trends: Vec::new(),
            optimization_opportunities: Vec::new(),
        }
    }
}

// Additional structs for analytics
#[derive(Debug, Serialize, Deserialize)]
pub struct CostAnalyticsReport {
    pub total_cost: f32,
    pub total_requests: u64,
    pub average_cost_per_request: f32,
    pub cost_savings: f32,
    pub savings_percentage: f32,
    pub provider_breakdown: Vec<ProviderCostBreakdown>,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
    pub cost_trends: Vec<CostTrend>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderCostBreakdown {
    pub provider: String,
    pub total_cost: f32,
    pub total_requests: u64,
    pub average_cost_per_request: f32,
    pub cost_per_token: f32,
    pub efficiency_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub potential_savings: f32,
    pub implementation_effort: EffortLevel,
    pub priority: RecommendationPriority,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendationType {
    ProviderOptimization,
    PerformanceOptimization,
    BudgetOptimization,
    UsagePatternOptimization,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}