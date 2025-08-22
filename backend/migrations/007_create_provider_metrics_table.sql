-- Provider metrics table for tracking AI provider performance
-- Migration: 007_create_provider_metrics_table
-- Description: Create provider_metrics table for analytics and cost optimization

CREATE TABLE IF NOT EXISTS provider_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    avg_response_time_ms DECIMAL(8,2),
    success_rate DECIMAL(5,4),
    total_requests INTEGER DEFAULT 0,
    total_cost_usd DECIMAL(12,6) DEFAULT 0.0,
    date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_provider_metrics_provider ON provider_metrics(provider);
CREATE INDEX idx_provider_metrics_model ON provider_metrics(provider, model);
CREATE INDEX idx_provider_metrics_date ON provider_metrics(date);
CREATE INDEX idx_provider_metrics_provider_date ON provider_metrics(provider, date);

-- Unique constraint to prevent duplicate entries per provider/model/date
CREATE UNIQUE INDEX idx_provider_metrics_unique ON provider_metrics(provider, model, date);

-- Function to update provider metrics
CREATE OR REPLACE FUNCTION update_provider_metrics(
    p_provider VARCHAR(50),
    p_model VARCHAR(100),
    p_response_time_ms INTEGER,
    p_success BOOLEAN,
    p_cost_usd DECIMAL(12,6)
)
RETURNS VOID AS $$
DECLARE
    today_date DATE := CURRENT_DATE;
    current_avg_time DECIMAL(8,2);
    current_success_rate DECIMAL(5,4);
    current_requests INTEGER;
BEGIN
    -- Insert or update metrics for today
    INSERT INTO provider_metrics (provider, model, avg_response_time_ms, success_rate, total_requests, total_cost_usd, date)
    VALUES (p_provider, p_model, p_response_time_ms, CASE WHEN p_success THEN 1.0 ELSE 0.0 END, 1, p_cost_usd, today_date)
    ON CONFLICT (provider, model, date)
    DO UPDATE SET
        avg_response_time_ms = (provider_metrics.avg_response_time_ms * provider_metrics.total_requests + p_response_time_ms) / (provider_metrics.total_requests + 1),
        success_rate = (provider_metrics.success_rate * provider_metrics.total_requests + CASE WHEN p_success THEN 1.0 ELSE 0.0 END) / (provider_metrics.total_requests + 1),
        total_requests = provider_metrics.total_requests + 1,
        total_cost_usd = provider_metrics.total_cost_usd + p_cost_usd;
END;
$$ LANGUAGE plpgsql;