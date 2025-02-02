-- Migration Up

-- Create the execution_configs table
CREATE TABLE execution_configs (
    config_id TEXT NOT NULL,
    config_type VARCHAR(10) NOT NULL,
    default_configs TEXT[] DEFAULT '{}',
    config JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (config_type, config_id)
);

-- Create indexes
CREATE INDEX idx_execution_configs_config_type ON execution_configs (config_type);
CREATE INDEX idx_execution_configs_default_configs ON execution_configs USING GIN (default_configs);
CREATE INDEX idx_execution_configs_config ON execution_configs USING GIN (config);

-- Create a function to automatically update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_modified_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create a trigger to automatically update the updated_at timestamp
CREATE TRIGGER update_execution_configs_modtime
BEFORE UPDATE ON execution_configs
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();

-- Create a view for default configs
CREATE OR REPLACE VIEW default_configs AS
SELECT 
    config_id,
    config->>'fee_recipient' AS fee_recipient,
    config->>'gas_limit' AS gas_limit,
    config->>'min_value' AS min_value,
    (config->>'grace')::integer AS grace,
    config->'relays' AS relays
FROM 
    execution_configs
WHERE 
    config_type = 'default';

-- Create a view for proposer configs
CREATE OR REPLACE VIEW proposer_configs AS
SELECT 
    config_id AS proposer,
    default_configs,
    config->>'fee_recipient' AS fee_recipient,
    config->>'gas_limit' AS gas_limit,
    config->>'min_value' AS min_value,
    (config->>'grace')::integer AS grace,
    (config->>'reset_relays')::boolean AS reset_relays,
    config->'relays' AS relays
FROM 
    execution_configs
WHERE 
    config_type = 'proposer';
