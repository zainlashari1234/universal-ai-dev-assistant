-- User preferences table for storing user settings
-- Migration: 009_create_user_preferences_table
-- Description: Create user_preferences table for customizable user settings

CREATE TABLE IF NOT EXISTS user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    default_provider VARCHAR(50),
    default_model VARCHAR(100),
    max_tokens INTEGER DEFAULT 4000,
    temperature DECIMAL(3,2) DEFAULT 0.7,
    auto_save BOOLEAN DEFAULT true,
    create_backups BOOLEAN DEFAULT true,
    theme VARCHAR(20) DEFAULT 'dark',
    language VARCHAR(10) DEFAULT 'en',
    timezone VARCHAR(50) DEFAULT 'UTC',
    notifications JSONB DEFAULT '{"email": true, "push": true, "desktop": false}',
    editor_settings JSONB DEFAULT '{"fontSize": 14, "tabSize": 2, "wordWrap": true}',
    ai_settings JSONB DEFAULT '{"enableInlineCompletion": true, "enableCodeExplanation": true}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_user_preferences_user_id ON user_preferences(user_id);
CREATE UNIQUE INDEX idx_user_preferences_unique_user ON user_preferences(user_id);

-- Update trigger for updated_at
CREATE TRIGGER update_user_preferences_updated_at BEFORE UPDATE ON user_preferences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to get user preferences with defaults
CREATE OR REPLACE FUNCTION get_user_preferences(p_user_id UUID)
RETURNS JSONB AS $$
DECLARE
    preferences JSONB;
BEGIN
    SELECT to_jsonb(up.*) INTO preferences
    FROM user_preferences up
    WHERE up.user_id = p_user_id;
    
    -- If no preferences found, return defaults
    IF preferences IS NULL THEN
        preferences := jsonb_build_object(
            'user_id', p_user_id,
            'default_provider', 'openrouter',
            'default_model', 'gpt-4o-mini',
            'max_tokens', 4000,
            'temperature', 0.7,
            'auto_save', true,
            'create_backups', true,
            'theme', 'dark',
            'language', 'en',
            'timezone', 'UTC',
            'notifications', '{"email": true, "push": true, "desktop": false}',
            'editor_settings', '{"fontSize": 14, "tabSize": 2, "wordWrap": true}',
            'ai_settings', '{"enableInlineCompletion": true, "enableCodeExplanation": true}'
        );
    END IF;
    
    RETURN preferences;
END;
$$ LANGUAGE plpgsql;