-- Conversation State Storage Table
-- This table stores conversational context for the OpenAI Responses API
-- Run this SQL against your PostgreSQL/Supabase database before enabling conversation state storage

CREATE TABLE IF NOT EXISTS conversation_states (
    response_id TEXT PRIMARY KEY,
    input_items JSONB NOT NULL,
    created_at BIGINT NOT NULL,
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_conversation_states_created_at
    ON conversation_states(created_at);

CREATE INDEX IF NOT EXISTS idx_conversation_states_provider
    ON conversation_states(provider);

-- Optional: Add a policy for automatic cleanup of old conversations
-- Uncomment and adjust the retention period as needed
-- CREATE INDEX IF NOT EXISTS idx_conversation_states_updated_at
--     ON conversation_states(updated_at);

COMMENT ON TABLE conversation_states IS 'Stores conversation history for OpenAI Responses API continuity';
COMMENT ON COLUMN conversation_states.response_id IS 'Unique identifier for the conversation state';
COMMENT ON COLUMN conversation_states.input_items IS 'JSONB array of conversation messages and context';
COMMENT ON COLUMN conversation_states.created_at IS 'Unix timestamp (seconds) when the conversation started';
COMMENT ON COLUMN conversation_states.model IS 'Model name used for this conversation';
COMMENT ON COLUMN conversation_states.provider IS 'LLM provider (e.g., openai, anthropic, bedrock)';
