-- API keys table for secure provider key management
-- Migration: 008_create_api_keys_table
-- Description: Create api_keys table for encrypted storage of provider API keys

CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    key_name VARCHAR(100) NOT NULL,
    encrypted_key TEXT NOT NULL,
    key_hash VARCHAR(255) NOT NULL, -- For quick validation without decryption
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    usage_count INTEGER DEFAULT 0,
    monthly_limit INTEGER, -- Optional usage limit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_provider ON api_keys(provider);
CREATE INDEX idx_api_keys_user_provider ON api_keys(user_id, provider);
CREATE INDEX idx_api_keys_is_active ON api_keys(is_active);
CREATE INDEX idx_api_keys_last_used ON api_keys(last_used_at);

-- Unique constraint for user/provider/key_name combination
CREATE UNIQUE INDEX idx_api_keys_unique ON api_keys(user_id, provider, key_name) WHERE is_active = true;

-- Update trigger for updated_at
CREATE TRIGGER update_api_keys_updated_at BEFORE UPDATE ON api_keys
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to update API key usage
CREATE OR REPLACE FUNCTION update_api_key_usage(p_key_hash VARCHAR(255))
RETURNS VOID AS $$
BEGIN
    UPDATE api_keys 
    SET 
        last_used_at = NOW(),
        usage_count = usage_count + 1
    WHERE key_hash = p_key_hash AND is_active = true;
END;
$$ LANGUAGE plpgsql;

-- Function to check API key limits
CREATE OR REPLACE FUNCTION check_api_key_limit(p_key_hash VARCHAR(255))
RETURNS BOOLEAN AS $$
DECLARE
    current_usage INTEGER;
    monthly_limit INTEGER;
    usage_this_month INTEGER;
BEGIN
    SELECT ak.monthly_limit INTO monthly_limit
    FROM api_keys ak
    WHERE ak.key_hash = p_key_hash AND ak.is_active = true;
    
    -- If no limit set, allow usage
    IF monthly_limit IS NULL THEN
        RETURN true;
    END IF;
    
    -- Count usage this month
    SELECT COUNT(*)
    INTO usage_this_month
    FROM completion_logs cl
    JOIN api_keys ak ON ak.user_id = (SELECT user_id FROM api_keys WHERE key_hash = p_key_hash)
    WHERE DATE_TRUNC('month', cl.created_at) = DATE_TRUNC('month', NOW());
    
    RETURN usage_this_month < monthly_limit;
END;
$$ LANGUAGE plpgsql;