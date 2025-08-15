-- P0 Day-3: Artifacts table migration
-- Migration: 004_create_artifacts_table
-- Description: Create artifacts table for storing test outputs, logs, and generated files

CREATE TYPE artifact_type AS ENUM ('test_report', 'coverage', 'log', 'binary', 'documentation', 'benchmark', 'screenshot', 'other');
CREATE TYPE storage_type AS ENUM ('local', 's3', 'gcs', 'azure', 'database');

CREATE TABLE IF NOT EXISTS artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    file_path VARCHAR(1000),
    storage_type storage_type NOT NULL DEFAULT 'local',
    artifact_type artifact_type NOT NULL DEFAULT 'other',
    mime_type VARCHAR(100),
    size_bytes BIGINT NOT NULL DEFAULT 0,
    checksum_sha256 VARCHAR(64),
    content_preview TEXT, -- First few lines for text files
    download_url VARCHAR(1000),
    storage_metadata JSONB DEFAULT '{}',
    retention_until TIMESTAMPTZ, -- Auto-cleanup date
    is_public BOOLEAN NOT NULL DEFAULT false,
    download_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_artifacts_run_id ON artifacts(run_id);
CREATE INDEX idx_artifacts_project_id ON artifacts(project_id);
CREATE INDEX idx_artifacts_user_id ON artifacts(user_id);
CREATE INDEX idx_artifacts_artifact_type ON artifacts(artifact_type);
CREATE INDEX idx_artifacts_storage_type ON artifacts(storage_type);
CREATE INDEX idx_artifacts_created_at ON artifacts(created_at);
CREATE INDEX idx_artifacts_name ON artifacts(name);
CREATE INDEX idx_artifacts_size_bytes ON artifacts(size_bytes);
CREATE INDEX idx_artifacts_retention_until ON artifacts(retention_until);

-- Composite indexes for common queries
CREATE INDEX idx_artifacts_run_type ON artifacts(run_id, artifact_type);
CREATE INDEX idx_artifacts_project_type ON artifacts(project_id, artifact_type);

-- Full-text search on artifact names and content previews
CREATE INDEX idx_artifacts_search ON artifacts USING gin(to_tsvector('english', name || ' ' || COALESCE(content_preview, '')));

-- Update trigger
CREATE TRIGGER update_artifacts_updated_at BEFORE UPDATE ON artifacts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();