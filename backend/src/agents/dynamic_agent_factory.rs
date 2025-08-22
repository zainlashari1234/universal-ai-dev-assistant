use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;

use super::{AgentPersonality, PersonalityTemplates, AgentId, AgentInfo, AgentStatus, CommunicationPreferences};

/// Dynamic Agent Factory - AI-powered agent creation system
/// Creates specialized agents on-demand based on project needs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: Uuid,
    pub name: String,
    pub agent_type: AgentType,
    pub capabilities: Vec<String>,
    pub personality_template: AgentPersonality,
    pub resource_requirements: ResourceRequirements,
    pub creation_prompt: String,
    pub success_metrics: Vec<String>,
    pub lifecycle_config: LifecycleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    /// Permanent specialist with deep expertise
    Specialist {
        domain: String,
        expertise_level: ExpertiseLevel,
        specialization: Vec<String>,
    },
    /// Temporary agent for specific tasks
    Temporary {
        task_type: TaskType,
        estimated_duration: chrono::Duration,
        auto_destroy: bool,
    },
    /// Hybrid agent combining multiple capabilities
    Hybrid {
        primary_capability: String,
        secondary_capabilities: Vec<String>,
        adaptation_rate: f32,
    },
    /// Experimental agent for R&D
    Experimental {
        research_area: String,
        hypothesis: String,
        success_criteria: Vec<String>,
    },
    /// Swarm agent - part of a coordinated group
    Swarm {
        swarm_id: Uuid,
        role_in_swarm: SwarmRole,
        coordination_protocol: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpertiseLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Master,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    CodeGeneration,
    Testing,
    Documentation,
    Optimization,
    Migration,
    Integration,
    Analysis,
    Monitoring,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmRole {
    Coordinator,
    Worker,
    Specialist,
    Monitor,
    Communicator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub storage_mb: u64,
    pub network_bandwidth: u64,
    pub ai_model_access: Vec<String>,
    pub external_apis: Vec<String>,
    pub max_concurrent_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    pub max_lifetime: Option<chrono::Duration>,
    pub idle_timeout: chrono::Duration,
    pub auto_scale: bool,
    pub backup_frequency: Option<chrono::Duration>,
    pub health_check_interval: chrono::Duration,
}

/// Dynamic Agent Factory - The brain of agent creation
pub struct DynamicAgentFactory {
    /// Available agent templates
    templates: Arc<RwLock<HashMap<String, AgentTemplate>>>,
    /// Currently active agents
    active_agents: Arc<RwLock<HashMap<AgentId, DynamicAgent>>>,
    /// Agent creation history
    creation_history: Arc<RwLock<Vec<AgentCreationRecord>>>,
    /// Resource pool
    resource_pool: Arc<RwLock<ResourcePool>>,
    /// AI-powered template generator
    template_generator: Arc<AITemplateGenerator>,
}

#[derive(Debug, Clone)]
pub struct DynamicAgent {
    pub info: AgentInfo,
    pub template: AgentTemplate,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub performance_metrics: PerformanceMetrics,
    pub lifecycle_state: LifecycleState,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCreationRecord {
    pub agent_id: AgentId,
    pub template_used: String,
    pub creation_reason: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<AgentId>,
    pub success: bool,
    pub performance_score: Option<f32>,
    pub destroyed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub available_cpu: f32,
    pub available_memory: u64,
    pub available_storage: u64,
    pub max_agents: usize,
    pub current_agents: usize,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub tasks_completed: u64,
    pub success_rate: f32,
    pub average_response_time: chrono::Duration,
    pub resource_efficiency: f32,
    pub collaboration_score: f32,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleState {
    Initializing,
    Active,
    Idle,
    Scaling,
    Maintenance,
    Terminating,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub storage_usage: u64,
    pub network_usage: u64,
}

/// AI-powered template generator
pub struct AITemplateGenerator {
    model_client: Arc<dyn AIModelClient>,
    template_cache: Arc<RwLock<HashMap<String, AgentTemplate>>>,
}

#[async_trait::async_trait]
pub trait AIModelClient: Send + Sync {
    async fn generate_agent_template(&self, request: &TemplateGenerationRequest) -> Result<AgentTemplate>;
    async fn optimize_agent_config(&self, agent: &DynamicAgent, metrics: &PerformanceMetrics) -> Result<AgentTemplate>;
    async fn predict_agent_needs(&self, project_context: &ProjectContext) -> Result<Vec<AgentType>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateGenerationRequest {
    pub task_description: String,
    pub required_capabilities: Vec<String>,
    pub performance_requirements: PerformanceRequirements,
    pub context: ProjectContext,
    pub constraints: CreationConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    pub response_time_ms: u64,
    pub accuracy_threshold: f32,
    pub throughput_per_hour: u64,
    pub availability_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: String,
    pub technologies: Vec<String>,
    pub team_size: usize,
    pub deadline: Option<DateTime<Utc>>,
    pub complexity_level: ComplexityLevel,
    pub existing_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,
    Moderate,
    Complex,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationConstraints {
    pub max_resources: ResourceRequirements,
    pub budget_limit: Option<f64>,
    pub security_level: SecurityLevel,
    pub compliance_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl DynamicAgentFactory {
    pub fn new(ai_client: Arc<dyn AIModelClient>) -> Self {
        let mut templates = HashMap::new();
        
        // Load predefined templates
        templates.insert("react_specialist".to_string(), Self::create_react_specialist_template());
        templates.insert("database_optimizer".to_string(), Self::create_database_optimizer_template());
        templates.insert("api_tester".to_string(), Self::create_api_tester_template());
        templates.insert("performance_monitor".to_string(), Self::create_performance_monitor_template());
        templates.insert("documentation_writer".to_string(), Self::create_documentation_writer_template());

        Self {
            templates: Arc::new(RwLock::new(templates)),
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            creation_history: Arc::new(RwLock::new(Vec::new())),
            resource_pool: Arc::new(RwLock::new(ResourcePool {
                available_cpu: 16.0,
                available_memory: 32768,
                available_storage: 1024000,
                max_agents: 50,
                current_agents: 0,
            })),
            template_generator: Arc::new(AITemplateGenerator {
                model_client: ai_client,
                template_cache: Arc::new(RwLock::new(HashMap::new())),
            }),
        }
    }

    /// Create an agent dynamically based on requirements
    pub async fn create_agent(&self, request: AgentCreationRequest) -> Result<AgentId> {
        // Check resource availability
        self.check_resource_availability(&request.resource_requirements).await?;

        // Find or generate appropriate template
        let template = self.find_or_generate_template(&request).await?;

        // Create agent instance
        let agent_id = AgentId {
            name: format!("{}_{}", template.name, Uuid::new_v4().to_string()[..8].to_string()),
            instance_id: Uuid::new_v4(),
            agent_type: template.agent_type.to_string(),
        };

        let agent_info = AgentInfo {
            id: agent_id.clone(),
            capabilities: template.capabilities.clone(),
            current_load: 0.0,
            last_heartbeat: Utc::now(),
            status: AgentStatus::Available,
            communication_preferences: CommunicationPreferences::default(),
        };

        let dynamic_agent = DynamicAgent {
            info: agent_info,
            template: template.clone(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            performance_metrics: PerformanceMetrics::default(),
            lifecycle_state: LifecycleState::Initializing,
            resource_usage: ResourceUsage::default(),
        };

        // Register agent
        {
            let mut agents = self.active_agents.write().await;
            agents.insert(agent_id.clone(), dynamic_agent);
        }

        // Update resource pool
        {
            let mut pool = self.resource_pool.write().await;
            pool.current_agents += 1;
            pool.available_cpu -= template.resource_requirements.cpu_cores;
            pool.available_memory -= template.resource_requirements.memory_mb;
        }

        // Record creation
        let creation_record = AgentCreationRecord {
            agent_id: agent_id.clone(),
            template_used: template.name.clone(),
            creation_reason: request.reason,
            created_at: Utc::now(),
            created_by: request.created_by,
            success: true,
            performance_score: None,
            destroyed_at: None,
        };

        {
            let mut history = self.creation_history.write().await;
            history.push(creation_record);
        }

        println!("🤖 Created dynamic agent: {} with capabilities: {:?}", 
                agent_id.name, template.capabilities);

        Ok(agent_id)
    }

    /// Find existing template or generate new one
    async fn find_or_generate_template(&self, request: &AgentCreationRequest) -> Result<AgentTemplate> {
        // First, try to find existing template
        {
            let templates = self.templates.read().await;
            for template in templates.values() {
                if self.template_matches_requirements(template, request) {
                    return Ok(template.clone());
                }
            }
        }

        // Generate new template using AI
        let generation_request = TemplateGenerationRequest {
            task_description: request.task_description.clone(),
            required_capabilities: request.required_capabilities.clone(),
            performance_requirements: request.performance_requirements.clone(),
            context: request.context.clone(),
            constraints: request.constraints.clone(),
        };

        let new_template = self.template_generator.model_client
            .generate_agent_template(&generation_request).await?;

        // Cache the new template
        {
            let mut templates = self.templates.write().await;
            templates.insert(new_template.name.clone(), new_template.clone());
        }

        Ok(new_template)
    }

    /// Check if template matches requirements
    fn template_matches_requirements(&self, template: &AgentTemplate, request: &AgentCreationRequest) -> bool {
        // Check capabilities overlap
        let capability_match = request.required_capabilities.iter()
            .all(|req_cap| template.capabilities.iter()
                .any(|temp_cap| temp_cap.contains(req_cap)));

        // Check resource requirements
        let resource_match = template.resource_requirements.cpu_cores <= request.resource_requirements.cpu_cores
            && template.resource_requirements.memory_mb <= request.resource_requirements.memory_mb;

        capability_match && resource_match
    }

    /// Check if resources are available
    async fn check_resource_availability(&self, requirements: &ResourceRequirements) -> Result<()> {
        let pool = self.resource_pool.read().await;
        
        if pool.current_agents >= pool.max_agents {
            return Err(anyhow!("Maximum agent limit reached"));
        }

        if pool.available_cpu < requirements.cpu_cores {
            return Err(anyhow!("Insufficient CPU resources"));
        }

        if pool.available_memory < requirements.memory_mb {
            return Err(anyhow!("Insufficient memory resources"));
        }

        Ok(())
    }

    /// Destroy an agent and free resources
    pub async fn destroy_agent(&self, agent_id: &AgentId) -> Result<()> {
        let agent = {
            let mut agents = self.active_agents.write().await;
            agents.remove(agent_id)
        };

        if let Some(agent) = agent {
            // Free resources
            {
                let mut pool = self.resource_pool.write().await;
                pool.current_agents -= 1;
                pool.available_cpu += agent.template.resource_requirements.cpu_cores;
                pool.available_memory += agent.template.resource_requirements.memory_mb;
            }

            // Update creation record
            {
                let mut history = self.creation_history.write().await;
                if let Some(record) = history.iter_mut()
                    .find(|r| r.agent_id == *agent_id) {
                    record.destroyed_at = Some(Utc::now());
                }
            }

            println!("🗑️ Destroyed agent: {}", agent_id.name);
        }

        Ok(())
    }

    /// Get agent creation statistics
    pub async fn get_creation_stats(&self) -> Result<CreationStats> {
        let history = self.creation_history.read().await;
        let active_agents = self.active_agents.read().await;
        let pool = self.resource_pool.read().await;

        Ok(CreationStats {
            total_created: history.len(),
            currently_active: active_agents.len(),
            success_rate: history.iter().map(|r| if r.success { 1.0 } else { 0.0 }).sum::<f32>() / history.len() as f32,
            resource_utilization: ResourceUtilization {
                cpu_usage: (16.0 - pool.available_cpu) / 16.0,
                memory_usage: (32768 - pool.available_memory) as f32 / 32768.0,
                agent_slots_used: pool.current_agents as f32 / pool.max_agents as f32,
            },
            most_created_types: HashMap::new(), // TODO: Implement
        })
    }

    /// Predefined template creators
    fn create_react_specialist_template() -> AgentTemplate {
        AgentTemplate {
            id: Uuid::new_v4(),
            name: "React Specialist".to_string(),
            agent_type: AgentType::Specialist {
                domain: "Frontend Development".to_string(),
                expertise_level: ExpertiseLevel::Expert,
                specialization: vec!["React".to_string(), "TypeScript".to_string(), "Redux".to_string()],
            },
            capabilities: vec![
                "react_development".to_string(),
                "component_design".to_string(),
                "state_management".to_string(),
                "performance_optimization".to_string(),
            ],
            personality_template: PersonalityTemplates::natural_language_programmer(),
            resource_requirements: ResourceRequirements {
                cpu_cores: 2.0,
                memory_mb: 4096,
                storage_mb: 10240,
                network_bandwidth: 1000,
                ai_model_access: vec!["gpt-4".to_string()],
                external_apis: vec!["npm".to_string(), "github".to_string()],
                max_concurrent_tasks: 5,
            },
            creation_prompt: "Create a React specialist agent for modern frontend development".to_string(),
            success_metrics: vec![
                "component_reusability".to_string(),
                "performance_score".to_string(),
                "code_quality".to_string(),
            ],
            lifecycle_config: LifecycleConfig {
                max_lifetime: None,
                idle_timeout: chrono::Duration::hours(2),
                auto_scale: true,
                backup_frequency: Some(chrono::Duration::hours(6)),
                health_check_interval: chrono::Duration::minutes(5),
            },
        }
    }

    fn create_database_optimizer_template() -> AgentTemplate {
        AgentTemplate {
            id: Uuid::new_v4(),
            name: "Database Optimizer".to_string(),
            agent_type: AgentType::Specialist {
                domain: "Database Performance".to_string(),
                expertise_level: ExpertiseLevel::Expert,
                specialization: vec!["SQL".to_string(), "PostgreSQL".to_string(), "Query Optimization".to_string()],
            },
            capabilities: vec![
                "query_optimization".to_string(),
                "index_analysis".to_string(),
                "performance_tuning".to_string(),
                "schema_design".to_string(),
            ],
            personality_template: PersonalityTemplates::build_doctor(),
            resource_requirements: ResourceRequirements {
                cpu_cores: 3.0,
                memory_mb: 8192,
                storage_mb: 20480,
                network_bandwidth: 2000,
                ai_model_access: vec!["claude-3".to_string()],
                external_apis: vec!["postgresql".to_string()],
                max_concurrent_tasks: 3,
            },
            creation_prompt: "Create a database optimization specialist for performance tuning".to_string(),
            success_metrics: vec![
                "query_performance_improvement".to_string(),
                "resource_usage_reduction".to_string(),
            ],
            lifecycle_config: LifecycleConfig {
                max_lifetime: None,
                idle_timeout: chrono::Duration::hours(4),
                auto_scale: false,
                backup_frequency: Some(chrono::Duration::hours(12)),
                health_check_interval: chrono::Duration::minutes(10),
            },
        }
    }

    fn create_api_tester_template() -> AgentTemplate {
        AgentTemplate {
            id: Uuid::new_v4(),
            name: "API Tester".to_string(),
            agent_type: AgentType::Temporary {
                task_type: TaskType::Testing,
                estimated_duration: chrono::Duration::hours(8),
                auto_destroy: true,
            },
            capabilities: vec![
                "api_testing".to_string(),
                "load_testing".to_string(),
                "security_testing".to_string(),
                "test_automation".to_string(),
            ],
            personality_template: PersonalityTemplates::bug_fixer(),
            resource_requirements: ResourceRequirements {
                cpu_cores: 1.5,
                memory_mb: 2048,
                storage_mb: 5120,
                network_bandwidth: 5000,
                ai_model_access: vec!["gpt-3.5-turbo".to_string()],
                external_apis: vec!["postman".to_string(), "curl".to_string()],
                max_concurrent_tasks: 10,
            },
            creation_prompt: "Create a temporary API testing agent for comprehensive endpoint validation".to_string(),
            success_metrics: vec![
                "test_coverage".to_string(),
                "bug_detection_rate".to_string(),
            ],
            lifecycle_config: LifecycleConfig {
                max_lifetime: Some(chrono::Duration::hours(24)),
                idle_timeout: chrono::Duration::minutes(30),
                auto_scale: false,
                backup_frequency: None,
                health_check_interval: chrono::Duration::minutes(2),
            },
        }
    }

    fn create_performance_monitor_template() -> AgentTemplate {
        AgentTemplate {
            id: Uuid::new_v4(),
            name: "Performance Monitor".to_string(),
            agent_type: AgentType::Specialist {
                domain: "System Monitoring".to_string(),
                expertise_level: ExpertiseLevel::Advanced,
                specialization: vec!["Monitoring".to_string(), "Alerting".to_string(), "Analytics".to_string()],
            },
            capabilities: vec![
                "performance_monitoring".to_string(),
                "anomaly_detection".to_string(),
                "alerting".to_string(),
                "metrics_analysis".to_string(),
            ],
            personality_template: PersonalityTemplates::security_analyzer(),
            resource_requirements: ResourceRequirements {
                cpu_cores: 1.0,
                memory_mb: 1024,
                storage_mb: 2048,
                network_bandwidth: 500,
                ai_model_access: vec!["claude-3-haiku".to_string()],
                external_apis: vec!["prometheus".to_string(), "grafana".to_string()],
                max_concurrent_tasks: 20,
            },
            creation_prompt: "Create a performance monitoring agent for continuous system observation".to_string(),
            success_metrics: vec![
                "uptime_monitoring".to_string(),
                "alert_accuracy".to_string(),
            ],
            lifecycle_config: LifecycleConfig {
                max_lifetime: None,
                idle_timeout: chrono::Duration::hours(24),
                auto_scale: true,
                backup_frequency: Some(chrono::Duration::hours(1)),
                health_check_interval: chrono::Duration::seconds(30),
            },
        }
    }

    fn create_documentation_writer_template() -> AgentTemplate {
        AgentTemplate {
            id: Uuid::new_v4(),
            name: "Documentation Writer".to_string(),
            agent_type: AgentType::Temporary {
                task_type: TaskType::Documentation,
                estimated_duration: chrono::Duration::hours(16),
                auto_destroy: false,
            },
            capabilities: vec![
                "documentation_writing".to_string(),
                "api_documentation".to_string(),
                "user_guides".to_string(),
                "code_comments".to_string(),
            ],
            personality_template: PersonalityTemplates::natural_language_programmer(),
            resource_requirements: ResourceRequirements {
                cpu_cores: 1.0,
                memory_mb: 2048,
                storage_mb: 5120,
                network_bandwidth: 1000,
                ai_model_access: vec!["gpt-4".to_string()],
                external_apis: vec!["github".to_string(), "confluence".to_string()],
                max_concurrent_tasks: 3,
            },
            creation_prompt: "Create a documentation specialist for comprehensive project documentation".to_string(),
            success_metrics: vec![
                "documentation_completeness".to_string(),
                "readability_score".to_string(),
            ],
            lifecycle_config: LifecycleConfig {
                max_lifetime: Some(chrono::Duration::days(7)),
                idle_timeout: chrono::Duration::hours(6),
                auto_scale: false,
                backup_frequency: Some(chrono::Duration::hours(4)),
                health_check_interval: chrono::Duration::minutes(15),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCreationRequest {
    pub task_description: String,
    pub required_capabilities: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub performance_requirements: PerformanceRequirements,
    pub context: ProjectContext,
    pub constraints: CreationConstraints,
    pub reason: String,
    pub created_by: Option<AgentId>,
}

#[derive(Debug, Serialize)]
pub struct CreationStats {
    pub total_created: usize,
    pub currently_active: usize,
    pub success_rate: f32,
    pub resource_utilization: ResourceUtilization,
    pub most_created_types: HashMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct ResourceUtilization {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub agent_slots_used: f32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            tasks_completed: 0,
            success_rate: 0.0,
            average_response_time: chrono::Duration::milliseconds(0),
            resource_efficiency: 0.0,
            collaboration_score: 0.0,
            learning_rate: 0.0,
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            storage_usage: 0,
            network_usage: 0,
        }
    }
}

impl ToString for AgentType {
    fn to_string(&self) -> String {
        match self {
            AgentType::Specialist { domain, .. } => format!("specialist_{}", domain.to_lowercase().replace(" ", "_")),
            AgentType::Temporary { task_type, .. } => format!("temporary_{:?}", task_type).to_lowercase(),
            AgentType::Hybrid { primary_capability, .. } => format!("hybrid_{}", primary_capability.to_lowercase()),
            AgentType::Experimental { research_area, .. } => format!("experimental_{}", research_area.to_lowercase().replace(" ", "_")),
            AgentType::Swarm { role_in_swarm, .. } => format!("swarm_{:?}", role_in_swarm).to_lowercase(),
        }
    }
}