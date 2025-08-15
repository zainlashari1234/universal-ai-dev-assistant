-- P0 Day-3: Completion logs table migration
-- Migration: 005_create_completion_logs_table  
-- Description: Create completion_logs table for tracking AI completion requests and responses

CREATE TYPE completion_provider AS ENUM ('openai', 'anthropic', 'ollama', 'heuristic', 'custom');
CREATE TYPE completion_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'timeout', 'cancelled');

CREATE TABLE IF NOT EXISTS completion_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    session_id UUID, -- For grouping related completions
    provider completion_provider NOT NULL,
    model_name VARCHAR(100),
    prompt_text TEXT NOT NULL,
    prompt_tokens INTEGER,
    completion_text TEXT,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    status completion_status NOT NULL DEFAULT 'pending',
    confidence_score REAL,
    language VARCHAR(50),
    context_size INTEGER,
    processing_time_ms BIGINT,
    cost_cents INTEGER, -- Cost in cents for tracking
    error_message TEXT,
    request_metadata JSONB DEFAULT '{}',
    response_metadata JSONB DEFAULT '{}',
    feedback_score INTEGER, -- User feedback (1-5)
    feedback_comment TEXT,
    is_accepted BOOLEAN, -- Whether user accepted the completion
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance and analytics
CREATE INDEX idx_completion_logs_user_id ON completion_logs(user_id);
CREATE INDEX idx_completion_logs_project_id ON completion_logs(project_id);
CREATE INDEX idx_completion_logs_session_id ON completion_logs(session_id);
CREATE INDEX idx_completion_logs_provider ON completion_logs(provider);
CREATE INDEX idx_completion_logs_model_name ON completion_logs(model_name);
CREATE INDEX idx_completion_logs_status ON completion_logs(status);
CREATE INDEX idx_completion_logs_language ON completion_logs(language);
CREATE INDEX idx_completion_logs_created_at ON completion_logs(created_at);
CREATE INDEX idx_completion_logs_confidence_score ON completion_logs(confidence_score);
CREATE INDEX idx_completion_logs_is_accepted ON completion_logs(is_accepted);

-- Composite indexes for analytics queries
CREATE INDEX idx_completion_logs_user_provider ON completion_logs(user_id, provider);
CREATE INDEX idx_completion_logs_project_language ON completion_logs(project_id, language);
CREATE INDEX idx_completion_logs_date_provider ON completion_logs(DATE(created_at), provider);

-- Partial indexes for performance
CREATE INDEX idx_completion_logs_successful ON completion_logs(created_at) WHERE status = 'completed';
CREATE INDEX idx_completion_logs_accepted ON completion_logs(created_at) WHERE is_accepted = true;

-- Update trigger
CREATE TRIGGER update_completion_logs_updated_at BEFORE UPDATE ON completion_logs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();