-- Migration Down

-- Drop the views
DROP VIEW IF EXISTS proposer_configs;
DROP VIEW IF EXISTS default_configs;

-- Drop the trigger
DROP TRIGGER IF EXISTS update_execution_configs_modtime ON execution_configs;

-- Drop the function
DROP FUNCTION IF EXISTS update_modified_column();

-- Drop the indexes
DROP INDEX IF EXISTS idx_execution_configs_config;
DROP INDEX IF EXISTS idx_execution_configs_default_configs;
DROP INDEX IF EXISTS idx_execution_configs_config_type;

-- Drop the table
DROP TABLE IF EXISTS execution_configs;
