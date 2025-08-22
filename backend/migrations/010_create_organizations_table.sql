-- Organizations table for multi-tenant support
-- Migration: 010_create_organizations_table
-- Description: Create organizations and organization_members tables

CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_tier VARCHAR(50) NOT NULL DEFAULT 'free',
    max_users INTEGER,
    max_api_calls_per_month BIGINT,
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Organization members table
CREATE TABLE IF NOT EXISTS organization_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'developer',
    invited_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, user_id)
);

-- Indexes for performance
CREATE INDEX idx_organizations_owner_id ON organizations(owner_id);
CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organization_members_org_id ON organization_members(organization_id);
CREATE INDEX idx_organization_members_user_id ON organization_members(user_id);

-- Update triggers
CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_members_updated_at BEFORE UPDATE ON organization_members
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to get user's organization
CREATE OR REPLACE FUNCTION get_user_organization(p_user_id UUID)
RETURNS UUID AS $$
DECLARE
    org_id UUID;
BEGIN
    -- First try to get owned organization
    SELECT id INTO org_id
    FROM organizations
    WHERE owner_id = p_user_id
    ORDER BY created_at ASC
    LIMIT 1;
    
    -- If no owned organization, get first membership
    IF org_id IS NULL THEN
        SELECT organization_id INTO org_id
        FROM organization_members
        WHERE user_id = p_user_id
        ORDER BY created_at ASC
        LIMIT 1;
    END IF;
    
    RETURN org_id;
END;
$$ LANGUAGE plpgsql;

-- Function to check organization limits
CREATE OR REPLACE FUNCTION check_organization_limits(p_org_id UUID)
RETURNS JSONB AS $$
DECLARE
    org_record RECORD;
    current_users INTEGER;
    current_api_calls BIGINT;
    result JSONB;
BEGIN
    -- Get organization details
    SELECT * INTO org_record
    FROM organizations
    WHERE id = p_org_id;
    
    IF NOT FOUND THEN
        RETURN jsonb_build_object('error', 'Organization not found');
    END IF;
    
    -- Count current users
    SELECT COUNT(*) INTO current_users
    FROM organization_members
    WHERE organization_id = p_org_id;
    
    -- Add owner to count
    current_users := current_users + 1;
    
    -- Count API calls this month
    SELECT COUNT(*) INTO current_api_calls
    FROM completion_logs cl
    JOIN runs r ON cl.run_id = r.id
    JOIN users u ON r.user_id = u.id
    LEFT JOIN organization_members om ON u.id = om.user_id AND om.organization_id = p_org_id
    WHERE (u.id = org_record.owner_id OR om.user_id IS NOT NULL)
    AND DATE_TRUNC('month', cl.created_at) = DATE_TRUNC('month', NOW());
    
    -- Build result
    result := jsonb_build_object(
        'organization_id', p_org_id,
        'subscription_tier', org_record.subscription_tier,
        'current_users', current_users,
        'max_users', org_record.max_users,
        'current_api_calls', current_api_calls,
        'max_api_calls', org_record.max_api_calls_per_month,
        'users_limit_reached', CASE 
            WHEN org_record.max_users IS NULL THEN false
            ELSE current_users >= org_record.max_users
        END,
        'api_calls_limit_reached', CASE
            WHEN org_record.max_api_calls_per_month IS NULL THEN false
            ELSE current_api_calls >= org_record.max_api_calls_per_month
        END
    );
    
    RETURN result;
END;
$$ LANGUAGE plpgsql;