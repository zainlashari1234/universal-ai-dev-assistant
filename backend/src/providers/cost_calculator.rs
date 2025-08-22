use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCalculation {
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub cost_per_token: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostComparison {
    pub calculations: Vec<CostCalculation>,
    pub cheapest: CostCalculation,
    pub most_expensive: CostCalculation,
    pub average_cost: f64,
    pub savings_potential: f64,
}

pub struct CostCalculator {
    pricing_data: HashMap<String, HashMap<String, ModelPricing>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_cost_per_1k_tokens: f64,
    pub output_cost_per_1k_tokens: f64,
    pub context_window: u32,
    pub max_output_tokens: u32,
}

impl CostCalculator {
    pub fn new() -> Self {
        let mut pricing_data = HashMap::new();
        
        // OpenAI pricing
        let mut openai_models = HashMap::new();
        openai_models.insert("gpt-4o".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.005,
            output_cost_per_1k_tokens: 0.015,
            context_window: 128000,
            max_output_tokens: 4096,
        });
        openai_models.insert("gpt-4o-mini".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.00015,
            output_cost_per_1k_tokens: 0.0006,
            context_window: 128000,
            max_output_tokens: 16384,
        });
        openai_models.insert("gpt-3.5-turbo".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.001,
            output_cost_per_1k_tokens: 0.002,
            context_window: 16385,
            max_output_tokens: 4096,
        });
        pricing_data.insert("openai".to_string(), openai_models);

        // Anthropic pricing
        let mut anthropic_models = HashMap::new();
        anthropic_models.insert("claude-3-sonnet".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.003,
            output_cost_per_1k_tokens: 0.015,
            context_window: 200000,
            max_output_tokens: 4096,
        });
        anthropic_models.insert("claude-3-haiku".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.00025,
            output_cost_per_1k_tokens: 0.00125,
            context_window: 200000,
            max_output_tokens: 4096,
        });
        pricing_data.insert("anthropic".to_string(), anthropic_models);

        // Google pricing
        let mut google_models = HashMap::new();
        google_models.insert("gemini-pro".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.001,
            output_cost_per_1k_tokens: 0.002,
            context_window: 32768,
            max_output_tokens: 8192,
        });
        google_models.insert("gemini-flash".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.0001,
            output_cost_per_1k_tokens: 0.0002,
            context_window: 32768,
            max_output_tokens: 8192,
        });
        pricing_data.insert("google".to_string(), google_models);

        // OpenRouter pricing (average)
        let mut openrouter_models = HashMap::new();
        openrouter_models.insert("gpt-4o".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.005,
            output_cost_per_1k_tokens: 0.015,
            context_window: 128000,
            max_output_tokens: 4096,
        });
        openrouter_models.insert("claude-3-sonnet".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.003,
            output_cost_per_1k_tokens: 0.015,
            context_window: 200000,
            max_output_tokens: 4096,
        });
        openrouter_models.insert("llama-3.1-70b".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.0009,
            output_cost_per_1k_tokens: 0.0009,
            context_window: 131072,
            max_output_tokens: 4096,
        });
        pricing_data.insert("openrouter".to_string(), openrouter_models);

        // Groq pricing (very cheap)
        let mut groq_models = HashMap::new();
        groq_models.insert("llama-3.1-70b".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.0001,
            output_cost_per_1k_tokens: 0.0001,
            context_window: 131072,
            max_output_tokens: 4096,
        });
        groq_models.insert("mixtral-8x7b".to_string(), ModelPricing {
            input_cost_per_1k_tokens: 0.00005,
            output_cost_per_1k_tokens: 0.00005,
            context_window: 32768,
            max_output_tokens: 4096,
        });
        pricing_data.insert("groq".to_string(), groq_models);

        Self { pricing_data }
    }

    pub fn calculate_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<CostCalculation> {
        let provider_models = self.pricing_data.get(provider)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider))?;
        
        let pricing = provider_models.get(model)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {} for provider: {}", model, provider))?;

        let input_cost = (input_tokens as f64 / 1000.0) * pricing.input_cost_per_1k_tokens;
        let output_cost = (output_tokens as f64 / 1000.0) * pricing.output_cost_per_1k_tokens;
        let total_cost = input_cost + output_cost;
        let total_tokens = input_tokens + output_tokens;
        let cost_per_token = if total_tokens > 0 { total_cost / total_tokens as f64 } else { 0.0 };

        Ok(CostCalculation {
            provider: provider.to_string(),
            model: model.to_string(),
            input_tokens,
            output_tokens,
            input_cost,
            output_cost,
            total_cost,
            cost_per_token,
        })
    }

    pub fn compare_costs(
        &self,
        providers_models: Vec<(String, String)>,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<CostComparison> {
        let mut calculations = Vec::new();
        
        for (provider, model) in providers_models {
            if let Ok(calc) = self.calculate_cost(&provider, &model, input_tokens, output_tokens) {
                calculations.push(calc);
            }
        }

        if calculations.is_empty() {
            return Err(anyhow::anyhow!("No valid provider/model combinations found"));
        }

        let cheapest = calculations.iter()
            .min_by(|a, b| a.total_cost.partial_cmp(&b.total_cost).unwrap())
            .unwrap()
            .clone();

        let most_expensive = calculations.iter()
            .max_by(|a, b| a.total_cost.partial_cmp(&b.total_cost).unwrap())
            .unwrap()
            .clone();

        let average_cost = calculations.iter()
            .map(|c| c.total_cost)
            .sum::<f64>() / calculations.len() as f64;

        let savings_potential = most_expensive.total_cost - cheapest.total_cost;

        Ok(CostComparison {
            calculations,
            cheapest,
            most_expensive,
            average_cost,
            savings_potential,
        })
    }

    pub fn estimate_monthly_cost(
        &self,
        provider: &str,
        model: &str,
        daily_requests: u32,
        avg_input_tokens: u32,
        avg_output_tokens: u32,
    ) -> Result<f64> {
        let daily_cost = self.calculate_cost(provider, model, 
            daily_requests * avg_input_tokens, 
            daily_requests * avg_output_tokens)?;
        
        Ok(daily_cost.total_cost * 30.0) // 30 days
    }

    pub fn get_cheapest_alternative(
        &self,
        current_provider: &str,
        current_model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<Option<CostCalculation>> {
        let current_cost = self.calculate_cost(current_provider, current_model, input_tokens, output_tokens)?;
        
        let mut best_alternative: Option<CostCalculation> = None;
        
        for (provider, models) in &self.pricing_data {
            for model in models.keys() {
                if provider == current_provider && model == current_model {
                    continue; // Skip current combination
                }
                
                if let Ok(calc) = self.calculate_cost(provider, model, input_tokens, output_tokens) {
                    if calc.total_cost < current_cost.total_cost {
                        match &best_alternative {
                            None => best_alternative = Some(calc),
                            Some(current_best) => {
                                if calc.total_cost < current_best.total_cost {
                                    best_alternative = Some(calc);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(best_alternative)
    }

    pub fn get_model_pricing(&self, provider: &str, model: &str) -> Option<&ModelPricing> {
        self.pricing_data.get(provider)?.get(model)
    }

    pub fn get_all_models(&self) -> Vec<(String, String, ModelPricing)> {
        let mut models = Vec::new();
        
        for (provider, provider_models) in &self.pricing_data {
            for (model, pricing) in provider_models {
                models.push((provider.clone(), model.clone(), pricing.clone()));
            }
        }
        
        models.sort_by(|a, b| {
            a.2.input_cost_per_1k_tokens.partial_cmp(&b.2.input_cost_per_1k_tokens).unwrap()
        });
        
        models
    }

    pub fn calculate_savings_report(
        &self,
        usage_history: Vec<(String, String, u32, u32)>, // (provider, model, input_tokens, output_tokens)
    ) -> Result<serde_json::Value> {
        let mut total_actual_cost = 0.0;
        let mut total_optimal_cost = 0.0;
        let mut provider_costs = HashMap::new();
        
        for (provider, model, input_tokens, output_tokens) in usage_history {
            let actual_cost = self.calculate_cost(&provider, &model, input_tokens, output_tokens)?;
            total_actual_cost += actual_cost.total_cost;
            
            // Find cheapest alternative
            if let Ok(Some(optimal)) = self.get_cheapest_alternative(&provider, &model, input_tokens, output_tokens) {
                total_optimal_cost += optimal.total_cost;
            } else {
                total_optimal_cost += actual_cost.total_cost;
            }
            
            *provider_costs.entry(provider).or_insert(0.0) += actual_cost.total_cost;
        }
        
        let potential_savings = total_actual_cost - total_optimal_cost;
        let savings_percentage = if total_actual_cost > 0.0 {
            (potential_savings / total_actual_cost) * 100.0
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "total_actual_cost": total_actual_cost,
            "total_optimal_cost": total_optimal_cost,
            "potential_savings": potential_savings,
            "savings_percentage": savings_percentage,
            "provider_breakdown": provider_costs,
            "recommendations": self.generate_cost_recommendations(&provider_costs)
        }))
    }

    fn generate_cost_recommendations(&self, provider_costs: &HashMap<String, f64>) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Find most expensive provider
        if let Some((most_expensive_provider, cost)) = provider_costs.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) {
            
            if *cost > 10.0 { // If spending more than $10
                recommendations.push(format!(
                    "Consider switching from {} to Groq for similar models at 90% lower cost",
                    most_expensive_provider
                ));
            }
        }
        
        // General recommendations
        recommendations.push("Use GPT-4o-mini instead of GPT-4o for simple tasks to save 75% on costs".to_string());
        recommendations.push("Consider Claude-3-Haiku for fast, cost-effective responses".to_string());
        recommendations.push("Use Groq for ultra-fast, low-cost inference when speed matters".to_string());
        
        recommendations
    }
}