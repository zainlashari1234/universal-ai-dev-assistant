-- P0 Day-3: Runs table migration
-- Migration: 003_create_runs_table
-- Description: Create runs table for tracking test executions and results

CREATE TYPE run_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled', 'timeout');
CREATE TYPE run_type AS ENUM ('test', 'build', 'deploy', 'analysis', 'benchmark');

CREATE TABLE IF NOT EXISTS runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    patch_id UUID, -- References patches (will be created later)
    plan_id UUID,  -- References plans (will be created later)
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    run_type run_type NOT NULL DEFAULT 'test',
    status run_status NOT NULL DEFAULT 'pending',
    command TEXT,
    environment JSONB DEFAULT '{}',
    working_directory VARCHAR(500),
    timeout_seconds INTEGER DEFAULT 300,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    exit_code INTEGER,
    stdout_log TEXT,
    stderr_log TEXT,
    test_results JSONB,
    coverage_data JSONB,
    performance_metrics JSONB,
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_runs_project_id ON runs(project_id);
CREATE INDEX idx_runs_user_id ON runs(user_id);
CREATE INDEX idx_runs_patch_id ON runs(patch_id);
CREATE INDEX idx_runs_plan_id ON runs(plan_id);
CREATE INDEX idx_runs_status ON runs(status);
CREATE INDEX idx_runs_run_type ON runs(run_type);
CREATE INDEX idx_runs_created_at ON runs(created_at);
CREATE INDEX idx_runs_started_at ON runs(started_at);
CREATE INDEX idx_runs_completed_at ON runs(completed_at);

-- Composite indexes for common queries
CREATE INDEX idx_runs_project_status ON runs(project_id, status);
CREATE INDEX idx_runs_user_status ON runs(user_id, status);

-- Update trigger
CREATE TRIGGER update_runs_updated_at BEFORE UPDATE ON runs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();