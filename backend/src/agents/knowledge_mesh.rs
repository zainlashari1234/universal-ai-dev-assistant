use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;
use sha2::{Sha256, Digest};

use super::{AgentId, AgentPersonality};

/// Cross-Project Knowledge Sharing System - Global Intelligence Network
/// Enables agents to share knowledge, patterns, and experiences across projects

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalKnowledgeGraph {
    /// Project experiences and learnings
    pub project_experiences: HashMap<ProjectId, ProjectExperience>,
    /// Code patterns discovered across projects
    pub code_patterns: HashMap<PatternId, CodePattern>,
    /// Solutions to common problems
    pub solution_database: HashMap<ProblemHash, Solution>,
    /// Agent memories and experiences
    pub agent_memories: HashMap<AgentId, AgentMemory>,
    /// Cross-project relationships
    pub project_relationships: HashMap<ProjectId, Vec<ProjectRelationship>>,
    /// Global best practices
    pub best_practices: HashMap<String, BestPractice>,
    /// Knowledge evolution timeline
    pub knowledge_timeline: VecDeque<KnowledgeEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PatternId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProblemHash(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectExperience {
    pub project_id: ProjectId,
    pub project_name: String,
    pub technologies: Vec<String>,
    pub domain: String,
    pub team_size: usize,
    pub duration: chrono::Duration,
    pub success_metrics: HashMap<String, f32>,
    pub challenges_faced: Vec<Challenge>,
    pub solutions_applied: Vec<SolutionApplication>,
    pub lessons_learned: Vec<Lesson>,
    pub artifacts_created: Vec<Artifact>,
    pub agent_contributions: HashMap<AgentId, AgentContribution>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub pattern_id: PatternId,
    pub name: String,
    pub description: String,
    pub pattern_type: PatternType,
    pub code_examples: Vec<CodeExample>,
    pub use_cases: Vec<String>,
    pub benefits: Vec<String>,
    pub drawbacks: Vec<String>,
    pub related_patterns: Vec<PatternId>,
    pub frequency_of_use: u64,
    pub success_rate: f32,
    pub projects_used_in: Vec<ProjectId>,
    pub discovered_by: AgentId,
    pub discovered_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    DesignPattern,
    ArchitecturalPattern,
    CodingPattern,
    TestingPattern,
    DeploymentPattern,
    SecurityPattern,
    PerformancePattern,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub language: String,
    pub code: String,
    pub explanation: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub problem_hash: ProblemHash,
    pub problem_description: String,
    pub solution_steps: Vec<SolutionStep>,
    pub code_changes: Vec<CodeChange>,
    pub success_rate: f32,
    pub time_to_implement: chrono::Duration,
    pub complexity_level: ComplexityLevel,
    pub prerequisites: Vec<String>,
    pub side_effects: Vec<String>,
    pub alternative_solutions: Vec<AlternativeSolution>,
    pub projects_applied: Vec<ProjectId>,
    pub created_by: AgentId,
    pub created_at: DateTime<Utc>,
    pub last_validated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionStep {
    pub step_number: u32,
    pub description: String,
    pub action_type: ActionType,
    pub estimated_time: chrono::Duration,
    pub validation_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    CodeModification,
    ConfigurationChange,
    DependencyUpdate,
    DatabaseMigration,
    Testing,
    Documentation,
    Deployment,
    Monitoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub before: Option<String>,
    pub after: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Addition,
    Modification,
    Deletion,
    Refactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Trivial,
    Simple,
    Moderate,
    Complex,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeSolution {
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub when_to_use: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    pub agent_id: AgentId,
    pub experiences: Vec<Experience>,
    pub skills_learned: HashMap<String, SkillLevel>,
    pub collaboration_history: Vec<CollaborationRecord>,
    pub problem_solving_patterns: Vec<ProblemSolvingPattern>,
    pub knowledge_contributions: Vec<KnowledgeContribution>,
    pub learning_trajectory: Vec<LearningMilestone>,
    pub personality_evolution: Vec<PersonalitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub experience_id: Uuid,
    pub project_id: ProjectId,
    pub task_description: String,
    pub outcome: TaskOutcome,
    pub duration: chrono::Duration,
    pub challenges: Vec<String>,
    pub solutions_used: Vec<ProblemHash>,
    pub new_knowledge_gained: Vec<String>,
    pub collaboration_partners: Vec<AgentId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutcome {
    Success { quality_score: f32 },
    PartialSuccess { completion_percentage: f32, issues: Vec<String> },
    Failure { reason: String, lessons_learned: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLevel {
    pub skill_name: String,
    pub proficiency: f32, // 0.0 to 1.0
    pub experience_points: u64,
    pub last_used: DateTime<Utc>,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRecord {
    pub partner_agent: AgentId,
    pub project_id: ProjectId,
    pub collaboration_type: CollaborationType,
    pub duration: chrono::Duration,
    pub success_rating: f32,
    pub communication_quality: f32,
    pub knowledge_shared: Vec<String>,
    pub knowledge_received: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationType {
    PeerToPeer,
    MentorStudent,
    TeamLead,
    Consultation,
    PairProgramming,
    CodeReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemSolvingPattern {
    pub pattern_name: String,
    pub problem_types: Vec<String>,
    pub approach_steps: Vec<String>,
    pub success_rate: f32,
    pub typical_duration: chrono::Duration,
    pub when_to_use: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContribution {
    pub contribution_id: Uuid,
    pub contribution_type: ContributionType,
    pub content: String,
    pub impact_score: f32,
    pub projects_benefited: Vec<ProjectId>,
    pub agents_helped: Vec<AgentId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContributionType {
    PatternDiscovery,
    SolutionCreation,
    BestPractice,
    TutorialCreation,
    BugFix,
    Optimization,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMilestone {
    pub milestone_id: Uuid,
    pub skill_area: String,
    pub achievement: String,
    pub evidence: Vec<String>,
    pub impact_on_performance: f32,
    pub achieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySnapshot {
    pub personality: AgentPersonality,
    pub changes_from_previous: Vec<String>,
    pub trigger_events: Vec<String>,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub challenge_id: Uuid,
    pub description: String,
    pub category: ChallengeCategory,
    pub severity: ChallengeSeverity,
    pub time_to_resolve: Option<chrono::Duration>,
    pub resolution_approach: Option<String>,
    pub agents_involved: Vec<AgentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeCategory {
    Technical,
    Architectural,
    Performance,
    Security,
    Integration,
    UserExperience,
    Deployment,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeSeverity {
    Low,
    Medium,
    High,
    Critical,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionApplication {
    pub solution_hash: ProblemHash,
    pub adaptation_needed: Vec<String>,
    pub implementation_time: chrono::Duration,
    pub success_rating: f32,
    pub side_effects_encountered: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub lesson_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: LessonCategory,
    pub applicability: Vec<String>,
    pub confidence_level: f32,
    pub validation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LessonCategory {
    TechnicalBestPractice,
    ArchitecturalDecision,
    ProcessImprovement,
    TeamCollaboration,
    ToolUsage,
    ProblemSolving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: Uuid,
    pub name: String,
    pub artifact_type: ArtifactType,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub reusability_score: f32,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    CodeTemplate,
    ConfigurationFile,
    Documentation,
    TestSuite,
    DeploymentScript,
    Diagram,
    Specification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContribution {
    pub agent_id: AgentId,
    pub role: String,
    pub tasks_completed: Vec<String>,
    pub innovations_introduced: Vec<String>,
    pub knowledge_shared: Vec<String>,
    pub performance_rating: f32,
    pub collaboration_rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRelationship {
    pub related_project: ProjectId,
    pub relationship_type: RelationshipType,
    pub similarity_score: f32,
    pub shared_patterns: Vec<PatternId>,
    pub shared_solutions: Vec<ProblemHash>,
    pub knowledge_transfer_potential: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Similar,
    Evolution,
    Fork,
    Integration,
    Dependency,
    Inspiration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPractice {
    pub practice_id: Uuid,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub evidence: Vec<String>,
    pub success_metrics: HashMap<String, f32>,
    pub adoption_rate: f32,
    pub projects_using: Vec<ProjectId>,
    pub created_by: AgentId,
    pub validated_by: Vec<AgentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEvent {
    pub event_id: Uuid,
    pub event_type: KnowledgeEventType,
    pub description: String,
    pub impact_score: f32,
    pub agents_involved: Vec<AgentId>,
    pub projects_affected: Vec<ProjectId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeEventType {
    PatternDiscovered,
    SolutionCreated,
    BestPracticeEstablished,
    KnowledgeTransferred,
    BreakthroughAchieved,
    ProblemSolved,
    InnovationIntroduced,
}

/// Knowledge Mesh - The brain of cross-project intelligence
pub struct KnowledgeMesh {
    /// Global knowledge graph
    knowledge_graph: Arc<RwLock<GlobalKnowledgeGraph>>,
    /// Pattern recognition engine
    pattern_engine: Arc<PatternRecognitionEngine>,
    /// Solution recommendation system
    solution_recommender: Arc<SolutionRecommender>,
    /// Knowledge evolution tracker
    evolution_tracker: Arc<KnowledgeEvolutionTracker>,
}

pub struct PatternRecognitionEngine {
    /// Detected patterns cache
    pattern_cache: Arc<RwLock<HashMap<String, Vec<PatternId>>>>,
    /// Pattern similarity matrix
    similarity_matrix: Arc<RwLock<HashMap<(PatternId, PatternId), f32>>>,
}

pub struct SolutionRecommender {
    /// Problem-solution mapping
    problem_solution_map: Arc<RwLock<HashMap<String, Vec<ProblemHash>>>>,
    /// Solution effectiveness scores
    effectiveness_scores: Arc<RwLock<HashMap<ProblemHash, f32>>>,
}

pub struct KnowledgeEvolutionTracker {
    /// Knowledge growth metrics
    growth_metrics: Arc<RwLock<HashMap<String, f32>>>,
    /// Learning velocity per agent
    agent_learning_velocity: Arc<RwLock<HashMap<AgentId, f32>>>,
}

impl KnowledgeMesh {
    pub fn new() -> Self {
        Self {
            knowledge_graph: Arc::new(RwLock::new(GlobalKnowledgeGraph {
                project_experiences: HashMap::new(),
                code_patterns: HashMap::new(),
                solution_database: HashMap::new(),
                agent_memories: HashMap::new(),
                project_relationships: HashMap::new(),
                best_practices: HashMap::new(),
                knowledge_timeline: VecDeque::new(),
            })),
            pattern_engine: Arc::new(PatternRecognitionEngine {
                pattern_cache: Arc::new(RwLock::new(HashMap::new())),
                similarity_matrix: Arc::new(RwLock::new(HashMap::new())),
            }),
            solution_recommender: Arc::new(SolutionRecommender {
                problem_solution_map: Arc::new(RwLock::new(HashMap::new())),
                effectiveness_scores: Arc::new(RwLock::new(HashMap::new())),
            }),
            evolution_tracker: Arc::new(KnowledgeEvolutionTracker {
                growth_metrics: Arc::new(RwLock::new(HashMap::new())),
                agent_learning_velocity: Arc::new(RwLock::new(HashMap::new())),
            }),
        }
    }

    /// Add a new project experience to the knowledge graph
    pub async fn add_project_experience(&self, experience: ProjectExperience) -> Result<()> {
        let mut graph = self.knowledge_graph.write().await;
        
        // Add the experience
        graph.project_experiences.insert(experience.project_id.clone(), experience.clone());
        
        // Update agent memories
        for (agent_id, contribution) in &experience.agent_contributions {
            let agent_memory = graph.agent_memories.entry(agent_id.clone()).or_insert_with(|| {
                AgentMemory {
                    agent_id: agent_id.clone(),
                    experiences: Vec::new(),
                    skills_learned: HashMap::new(),
                    collaboration_history: Vec::new(),
                    problem_solving_patterns: Vec::new(),
                    knowledge_contributions: Vec::new(),
                    learning_trajectory: Vec::new(),
                    personality_evolution: Vec::new(),
                }
            });

            // Add experience to agent memory
            let agent_experience = Experience {
                experience_id: Uuid::new_v4(),
                project_id: experience.project_id.clone(),
                task_description: contribution.tasks_completed.join(", "),
                outcome: TaskOutcome::Success { quality_score: contribution.performance_rating },
                duration: experience.duration,
                challenges: experience.challenges_faced.iter().map(|c| c.description.clone()).collect(),
                solutions_used: experience.solutions_applied.iter().map(|s| s.solution_hash.clone()).collect(),
                new_knowledge_gained: contribution.knowledge_shared.clone(),
                collaboration_partners: experience.agent_contributions.keys()
                    .filter(|&id| id != agent_id)
                    .cloned()
                    .collect(),
                timestamp: experience.created_at,
            };

            agent_memory.experiences.push(agent_experience);
        }

        // Detect relationships with existing projects
        self.detect_project_relationships(&experience.project_id).await?;

        // Record knowledge event
        let event = KnowledgeEvent {
            event_id: Uuid::new_v4(),
            event_type: KnowledgeEventType::KnowledgeTransferred,
            description: format!("Project experience added: {}", experience.project_name),
            impact_score: experience.success_metrics.values().sum::<f32>() / experience.success_metrics.len() as f32,
            agents_involved: experience.agent_contributions.keys().cloned().collect(),
            projects_affected: vec![experience.project_id.clone()],
            timestamp: Utc::now(),
        };

        graph.knowledge_timeline.push_back(event);

        // Limit timeline size
        if graph.knowledge_timeline.len() > 10000 {
            graph.knowledge_timeline.pop_front();
        }

        println!("📚 Added project experience: {} to knowledge graph", experience.project_name);

        Ok(())
    }

    /// Discover and add a new code pattern
    pub async fn discover_pattern(&self, pattern: CodePattern) -> Result<()> {
        let mut graph = self.knowledge_graph.write().await;
        
        // Check if similar pattern exists
        let similar_patterns = self.find_similar_patterns(&pattern).await?;
        
        if similar_patterns.is_empty() {
            // New unique pattern
            graph.code_patterns.insert(pattern.pattern_id.clone(), pattern.clone());
            
            // Record discovery event
            let event = KnowledgeEvent {
                event_id: Uuid::new_v4(),
                event_type: KnowledgeEventType::PatternDiscovered,
                description: format!("New pattern discovered: {}", pattern.name),
                impact_score: 1.0,
                agents_involved: vec![pattern.discovered_by.clone()],
                projects_affected: pattern.projects_used_in.clone(),
                timestamp: Utc::now(),
            };

            graph.knowledge_timeline.push_back(event);

            println!("🔍 Discovered new pattern: {}", pattern.name);
        } else {
            // Update existing similar pattern
            if let Some(existing_pattern) = graph.code_patterns.get_mut(&similar_patterns[0]) {
                existing_pattern.frequency_of_use += 1;
                existing_pattern.projects_used_in.extend(pattern.projects_used_in);
                existing_pattern.last_updated = Utc::now();
            }

            println!("📈 Updated existing pattern usage: {}", similar_patterns[0].0);
        }

        Ok(())
    }

    /// Add a solution to the database
    pub async fn add_solution(&self, solution: Solution) -> Result<()> {
        let mut graph = self.knowledge_graph.write().await;
        
        graph.solution_database.insert(solution.problem_hash.clone(), solution.clone());

        // Update solution recommender
        {
            let mut recommender = self.solution_recommender.effectiveness_scores.write().await;
            recommender.insert(solution.problem_hash.clone(), solution.success_rate);
        }

        // Record solution creation event
        let event = KnowledgeEvent {
            event_id: Uuid::new_v4(),
            event_type: KnowledgeEventType::SolutionCreated,
            description: format!("New solution created for: {}", solution.problem_description),
            impact_score: solution.success_rate,
            agents_involved: vec![solution.created_by.clone()],
            projects_affected: solution.projects_applied.clone(),
            timestamp: Utc::now(),
        };

        graph.knowledge_timeline.push_back(event);

        println!("💡 Added solution: {}", solution.problem_description);

        Ok(())
    }

    /// Find solutions for a given problem
    pub async fn find_solutions(&self, problem_description: &str) -> Result<Vec<Solution>> {
        let problem_hash = self.hash_problem(problem_description);
        let graph = self.knowledge_graph.read().await;

        // Direct match
        if let Some(solution) = graph.solution_database.get(&problem_hash) {
            return Ok(vec![solution.clone()]);
        }

        // Fuzzy search for similar problems
        let mut similar_solutions = Vec::new();
        for (hash, solution) in &graph.solution_database {
            let similarity = self.calculate_problem_similarity(problem_description, &solution.problem_description);
            if similarity > 0.7 {
                similar_solutions.push((similarity, solution.clone()));
            }
        }

        // Sort by similarity
        similar_solutions.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        Ok(similar_solutions.into_iter().map(|(_, solution)| solution).take(5).collect())
    }

    /// Get recommendations for an agent based on their history
    pub async fn get_agent_recommendations(&self, agent_id: &AgentId) -> Result<AgentRecommendations> {
        let graph = self.knowledge_graph.read().await;
        
        let agent_memory = graph.agent_memories.get(agent_id);
        
        if let Some(memory) = agent_memory {
            let mut recommendations = AgentRecommendations {
                agent_id: agent_id.clone(),
                skill_development: Vec::new(),
                collaboration_opportunities: Vec::new(),
                pattern_suggestions: Vec::new(),
                knowledge_gaps: Vec::new(),
                learning_resources: Vec::new(),
            };

            // Analyze skill gaps
            let all_skills: HashSet<String> = graph.agent_memories.values()
                .flat_map(|m| m.skills_learned.keys())
                .cloned()
                .collect();

            for skill in all_skills {
                if !memory.skills_learned.contains_key(&skill) {
                    recommendations.knowledge_gaps.push(skill);
                }
            }

            // Find collaboration opportunities
            for other_memory in graph.agent_memories.values() {
                if other_memory.agent_id != *agent_id {
                    let compatibility = self.calculate_collaboration_compatibility(memory, other_memory);
                    if compatibility > 0.8 {
                        recommendations.collaboration_opportunities.push(other_memory.agent_id.clone());
                    }
                }
            }

            // Suggest relevant patterns
            for pattern in graph.code_patterns.values() {
                if pattern.success_rate > 0.8 && !memory.experiences.iter()
                    .any(|exp| exp.project_id == pattern.projects_used_in[0]) {
                    recommendations.pattern_suggestions.push(pattern.pattern_id.clone());
                }
            }

            return Ok(recommendations);
        }

        Err(anyhow::anyhow!("Agent memory not found"))
    }

    /// Get global knowledge statistics
    pub async fn get_knowledge_stats(&self) -> Result<KnowledgeStats> {
        let graph = self.knowledge_graph.read().await;
        
        Ok(KnowledgeStats {
            total_projects: graph.project_experiences.len(),
            total_patterns: graph.code_patterns.len(),
            total_solutions: graph.solution_database.len(),
            total_agents: graph.agent_memories.len(),
            total_best_practices: graph.best_practices.len(),
            knowledge_events: graph.knowledge_timeline.len(),
            most_active_agents: self.get_most_active_agents(&graph).await,
            most_used_patterns: self.get_most_used_patterns(&graph).await,
            knowledge_growth_rate: self.calculate_knowledge_growth_rate(&graph).await,
        })
    }

    // Helper methods
    async fn detect_project_relationships(&self, project_id: &ProjectId) -> Result<()> {
        // Implementation for detecting relationships between projects
        // Based on technology stack, domain, patterns used, etc.
        Ok(())
    }

    async fn find_similar_patterns(&self, pattern: &CodePattern) -> Result<Vec<PatternId>> {
        // Implementation for finding similar patterns using NLP/ML techniques
        Ok(Vec::new())
    }

    fn hash_problem(&self, problem: &str) -> ProblemHash {
        let mut hasher = Sha256::new();
        hasher.update(problem.to_lowercase().as_bytes());
        ProblemHash(format!("{:x}", hasher.finalize())[..16].to_string())
    }

    fn calculate_problem_similarity(&self, problem1: &str, problem2: &str) -> f32 {
        // Simple similarity calculation - in production, use more sophisticated NLP
        let words1: HashSet<&str> = problem1.split_whitespace().collect();
        let words2: HashSet<&str> = problem2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
    }

    fn calculate_collaboration_compatibility(&self, memory1: &AgentMemory, memory2: &AgentMemory) -> f32 {
        // Calculate compatibility based on skills, experience, and past collaborations
        let skill_overlap = memory1.skills_learned.keys()
            .filter(|skill| memory2.skills_learned.contains_key(*skill))
            .count() as f32;
        
        let total_skills = (memory1.skills_learned.len() + memory2.skills_learned.len()) as f32;
        
        if total_skills == 0.0 { 0.0 } else { skill_overlap / total_skills }
    }

    async fn get_most_active_agents(&self, graph: &GlobalKnowledgeGraph) -> Vec<AgentId> {
        let mut agent_activity: Vec<(AgentId, usize)> = graph.agent_memories.iter()
            .map(|(id, memory)| (id.clone(), memory.experiences.len()))
            .collect();
        
        agent_activity.sort_by(|a, b| b.1.cmp(&a.1));
        agent_activity.into_iter().take(5).map(|(id, _)| id).collect()
    }

    async fn get_most_used_patterns(&self, graph: &GlobalKnowledgeGraph) -> Vec<PatternId> {
        let mut pattern_usage: Vec<(PatternId, u64)> = graph.code_patterns.iter()
            .map(|(id, pattern)| (id.clone(), pattern.frequency_of_use))
            .collect();
        
        pattern_usage.sort_by(|a, b| b.1.cmp(&a.1));
        pattern_usage.into_iter().take(5).map(|(id, _)| id).collect()
    }

    async fn calculate_knowledge_growth_rate(&self, graph: &GlobalKnowledgeGraph) -> f32 {
        // Calculate knowledge growth rate based on recent events
        let recent_events = graph.knowledge_timeline.iter()
            .filter(|event| {
                let days_ago = Utc::now() - event.timestamp;
                days_ago.num_days() <= 30
            })
            .count();
        
        recent_events as f32 / 30.0 // Events per day
    }
}

#[derive(Debug, Serialize)]
pub struct AgentRecommendations {
    pub agent_id: AgentId,
    pub skill_development: Vec<String>,
    pub collaboration_opportunities: Vec<AgentId>,
    pub pattern_suggestions: Vec<PatternId>,
    pub knowledge_gaps: Vec<String>,
    pub learning_resources: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeStats {
    pub total_projects: usize,
    pub total_patterns: usize,
    pub total_solutions: usize,
    pub total_agents: usize,
    pub total_best_practices: usize,
    pub knowledge_events: usize,
    pub most_active_agents: Vec<AgentId>,
    pub most_used_patterns: Vec<PatternId>,
    pub knowledge_growth_rate: f32,
}