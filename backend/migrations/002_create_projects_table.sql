-- P0 Day-3: Projects table migration
-- Migration: 002_create_projects_table
-- Description: Create projects table for workspace/project management

CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    repository_url VARCHAR(500),
    local_path VARCHAR(500),
    language VARCHAR(50),
    framework VARCHAR(100),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_public BOOLEAN NOT NULL DEFAULT false,
    is_archived BOOLEAN NOT NULL DEFAULT false,
    settings JSONB DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_projects_owner_id ON projects(owner_id);
CREATE INDEX idx_projects_language ON projects(language);
CREATE INDEX idx_projects_is_public ON projects(is_public);
CREATE INDEX idx_projects_is_archived ON projects(is_archived);
CREATE INDEX idx_projects_created_at ON projects(created_at);
CREATE INDEX idx_projects_name ON projects(name);

-- Full-text search index for project search
CREATE INDEX idx_projects_search ON projects USING gin(to_tsvector('english', name || ' ' || COALESCE(description, '')));

-- Update trigger
CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();