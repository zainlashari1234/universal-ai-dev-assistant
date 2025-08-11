use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReview {
    pub id: Uuid,
    pub title: String,
    pub author: Uuid,
    pub reviewers: Vec<Uuid>,
    pub files: Vec<ReviewFile>,
    pub status: ReviewStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub ai_analysis: Option<AIReviewAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewStatus {
    Draft,
    InReview,
    Approved,
    ChangesRequested,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFile {
    pub path: String,
    pub original_content: String,
    pub modified_content: String,
    pub comments: Vec<ReviewComment>,
    pub ai_suggestions: Vec<AISuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: Uuid,
    pub author: Uuid,
    pub line: u32,
    pub content: String,
    pub comment_type: CommentType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentType {
    General,
    Issue,
    Suggestion,
    Question,
    Praise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISuggestion {
    pub id: Uuid,
    pub line: u32,
    pub suggestion_type: AISuggestionType,
    pub message: String,
    pub suggested_code: Option<String>,
    pub confidence: f64,
    pub reasoning: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AISuggestionType {
    Security,
    Performance,
    CodeQuality,
    BestPractice,
    Documentation,
    Testing,
    Refactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIReviewAnalysis {
    pub overall_score: f64,
    pub security_score: f64,
    pub performance_score: f64,
    pub maintainability_score: f64,
    pub test_coverage_score: f64,
    pub summary: String,
    pub key_issues: Vec<String>,
    pub recommendations: Vec<String>,
    pub estimated_review_time: u32, // minutes
}

pub struct AICodeReviewer {
    reviews: Arc<RwLock<HashMap<Uuid, CodeReview>>>,
    ai_engine: Option<Arc<RwLock<crate::ai_engine::AIEngine>>>,
}

impl AICodeReviewer {
    pub fn new(ai_engine: Option<Arc<RwLock<crate::ai_engine::AIEngine>>>) -> Self {
        Self {
            reviews: Arc::new(RwLock::new(HashMap::new())),
            ai_engine,
        }
    }

    pub async fn create_review(&self, title: String, author: Uuid, files: Vec<ReviewFile>) -> Result<Uuid> {
        let review_id = Uuid::new_v4();
        
        let mut review = CodeReview {
            id: review_id,
            title,
            author,
            reviewers: Vec::new(),
            files,
            status: ReviewStatus::Draft,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ai_analysis: None,
        };

        // Perform AI analysis
        if let Some(ai_engine) = &self.ai_engine {
            review.ai_analysis = Some(self.perform_ai_analysis(&review.files, ai_engine).await?);
        }

        let mut reviews = self.reviews.write().await;
        reviews.insert(review_id, review);

        info!("Created code review: {}", review_id);
        Ok(review_id)
    }

    pub async fn add_reviewer(&self, review_id: Uuid, reviewer_id: Uuid) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        
        if let Some(review) = reviews.get_mut(&review_id) {
            if !review.reviewers.contains(&reviewer_id) {
                review.reviewers.push(reviewer_id);
                review.updated_at = chrono::Utc::now();
                
                if review.status == ReviewStatus::Draft {
                    review.status = ReviewStatus::InReview;
                }
            }
        }

        Ok(())
    }

    pub async fn add_comment(&self, review_id: Uuid, comment: ReviewComment) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        
        if let Some(review) = reviews.get_mut(&review_id) {
            // Find the file and add comment
            for file in &mut review.files {
                if file.path == "target_file" { // Simplified - would match by file path
                    file.comments.push(comment);
                    review.updated_at = chrono::Utc::now();
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn get_ai_suggestions(&self, review_id: Uuid, file_path: String) -> Result<Vec<AISuggestion>> {
        let reviews = self.reviews.read().await;
        
        if let Some(review) = reviews.get(&review_id) {
            if let Some(file) = review.files.iter().find(|f| f.path == file_path) {
                return Ok(file.ai_suggestions.clone());
            }
        }

        Ok(Vec::new())
    }

    pub async fn accept_ai_suggestion(&self, review_id: Uuid, suggestion_id: Uuid) -> Result<()> {
        let mut reviews = self.reviews.write().await;
        
        if let Some(review) = reviews.get_mut(&review_id) {
            for file in &mut review.files {
                for suggestion in &mut file.ai_suggestions {
                    if suggestion.id == suggestion_id {
                        suggestion.accepted = true;
                        
                        // Apply the suggestion to the code
                        if let Some(suggested_code) = &suggestion.suggested_code {
                            // In a real implementation, this would apply the code change
                            info!("Applied AI suggestion: {}", suggested_code);
                        }
                        
                        review.updated_at = chrono::Utc::now();
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    async fn perform_ai_analysis(&self, files: &[ReviewFile], ai_engine: &Arc<RwLock<crate::ai_engine::AIEngine>>) -> Result<AIReviewAnalysis> {
        let mut overall_score = 0.0;
        let mut security_score = 0.0;
        let mut performance_score = 0.0;
        let mut maintainability_score = 0.0;
        let mut test_coverage_score = 0.0;
        let mut key_issues = Vec::new();
        let mut recommendations = Vec::new();

        for file in files {
            let analysis = self.analyze_file_changes(file, ai_engine).await?;
            
            overall_score += analysis.overall_score;
            security_score += analysis.security_score;
            performance_score += analysis.performance_score;
            maintainability_score += analysis.maintainability_score;
            test_coverage_score += analysis.test_coverage_score;
            
            key_issues.extend(analysis.key_issues);
            recommendations.extend(analysis.recommendations);
        }

        let file_count = files.len() as f64;
        overall_score /= file_count;
        security_score /= file_count;
        performance_score /= file_count;
        maintainability_score /= file_count;
        test_coverage_score /= file_count;

        let estimated_review_time = self.estimate_review_time(files);

        Ok(AIReviewAnalysis {
            overall_score,
            security_score,
            performance_score,
            maintainability_score,
            test_coverage_score,
            summary: self.generate_review_summary(&key_issues, &recommendations),
            key_issues,
            recommendations,
            estimated_review_time,
        })
    }

    async fn analyze_file_changes(&self, file: &ReviewFile, _ai_engine: &Arc<RwLock<crate::ai_engine::AIEngine>>) -> Result<AIReviewAnalysis> {
        // Simplified analysis - in real implementation would use AI engine
        let mut key_issues = Vec::new();
        let mut recommendations = Vec::new();

        // Security analysis
        if file.modified_content.contains("eval(") {
            key_issues.push("Security: Use of eval() detected".to_string());
        }
        
        if file.modified_content.contains("shell=True") {
            key_issues.push("Security: Shell injection risk".to_string());
        }

        // Performance analysis
        let nested_loops = file.modified_content.matches("for ").count();
        if nested_loops >= 2 {
            key_issues.push("Performance: Potential O(n²) complexity".to_string());
        }

        // Code quality
        if !file.modified_content.contains("\"\"\"") && file.modified_content.contains("def ") {
            recommendations.push("Add docstrings to functions".to_string());
        }

        // Test coverage
        let has_tests = file.path.contains("test") || file.modified_content.contains("test_");
        let test_coverage_score = if has_tests { 0.8 } else { 0.3 };

        Ok(AIReviewAnalysis {
            overall_score: 0.75,
            security_score: if key_issues.iter().any(|i| i.contains("Security")) { 0.4 } else { 0.9 },
            performance_score: if key_issues.iter().any(|i| i.contains("Performance")) { 0.5 } else { 0.8 },
            maintainability_score: 0.7,
            test_coverage_score,
            summary: String::new(),
            key_issues,
            recommendations,
            estimated_review_time: 0,
        })
    }

    fn estimate_review_time(&self, files: &[ReviewFile]) -> u32 {
        let mut total_time = 0;
        
        for file in files {
            let lines_changed = file.modified_content.lines().count();
            // Estimate 1 minute per 10 lines of code
            total_time += (lines_changed / 10) as u32 + 5; // Base 5 minutes per file
        }
        
        total_time
    }

    fn generate_review_summary(&self, key_issues: &[String], recommendations: &[String]) -> String {
        let mut summary = String::new();
        
        if key_issues.is_empty() {
            summary.push_str("✅ No critical issues found. ");
        } else {
            summary.push_str(&format!("⚠️ {} issues found. ", key_issues.len()));
        }
        
        if !recommendations.is_empty() {
            summary.push_str(&format!("💡 {} recommendations provided.", recommendations.len()));
        }
        
        summary
    }
}