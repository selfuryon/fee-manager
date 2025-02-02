DELETE FROM execution_configs;

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('mainnet', 'default', '{
  "fee_recipient": "0x0123456789abcdef0123456789abcdef01234567",
  "gas_limit": "30000000",
  "min_value": "0.1",
  "grace": 1000,
  "relays": {
    "https://relay1.com/": {
      "public_key": "0xac6e37ae5555555555555555555555555555555555555555555555555555555555",
      "min_value": "0.2",
      "fee_recipient": "0x1111111111111111111111111111111111111111",
      "gas_limit": "35000000"
    },
    "https://relay2.com/": {
      "public_key": "0x8b5d6b8f7777777777777777777777777777777777777777777777777777777777",
      "min_value": "0.15"
    }
  }
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('testnet', 'default', '{
  "fee_recipient": "0x9876543210abcdef9876543210abcdef98765432",
  "gas_limit": "25000000",
  "min_value": "0.05",
  "relays": {
    "https://testnet-relay1.com/": {
      "public_key": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "min_value": "0.1"
    }
  }
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('local', 'default', '{
  "fee_recipient": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "gas_limit": "20000000",
  "min_value": "0.01",
  "grace": 500,
  "relays": {}
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('minimal', 'default', '{
  "fee_recipient": "0xdddddddddddddddddddddddddddddddddddddddd",
  "gas_limit": "15000000",
  "min_value": "0.001",
  "relays": {}
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('multi-relay', 'default', '{
  "fee_recipient": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "gas_limit": "40000000",
  "min_value": "0.2",
  "grace": 2000,
  "relays": {
    "https://relay1.example.com/": {
      "public_key": "0x1111111111111111111111111111111111111111111111111111111111111111",
      "min_value": "0.3"
    },
    "https://relay2.example.com/": {
      "public_key": "0x2222222222222222222222222222222222222222222222222222222222222222",
      "fee_recipient": "0x3333333333333333333333333333333333333333"
    },
    "https://relay3.example.com/": {
      "public_key": "0x3333333333333333333333333333333333333333333333333333333333333333",
      "gas_limit": "45000000"
    },
    "https://relay4.example.com/": {
      "public_key": "0x4444444444444444444444444444444444444444444444444444444444444444",
      "min_value": "0.25",
      "fee_recipient": "0x4444444444444444444444444444444444444444",
      "gas_limit": "50000000"
    }
  }
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, config) VALUES
('high-gas', 'default', '{
  "fee_recipient": "0xffffffffffffffffffffffffffffffffffffffff",
  "gas_limit": "100000000",
  "min_value": "1.0",
  "grace": 5000,
  "relays": {
    "https://high-gas-relay.com/": {
      "public_key": "0x5555555555555555555555555555555555555555555555555555555555555555",
      "gas_limit": "120000000"
    }
  }
}'::jsonb);

INSERT INTO execution_configs (config_id, config_type, default_configs, config)
VALUES
    ('0x11111', 'proposer', '{}', '{"fee_recipient": "0xABC123ABC123ABC123ABC123ABC123ABC123ABC1", "gas_limit": "200000", "min_value": "0.5", "grace": 60, "relays": ["relay1", "relay2"]}'),
    ('0x22222', 'proposer', '{"config1", "config2"}', '{"fee_recipient": "0x789ABC789ABC789ABC789ABC789ABC789ABC789A", "gas_limit": "250000", "min_value": "0.9", "grace": 180, "reset_relays": true, "relays": ["relay5", "relay6"]}'),
    ('0x33333', 'proposer', '{"config2"}', '{"fee_recipient": "0x456DEF456DEF456DEF456DEF456DEF456DEF456D", "gas_limit": "500000", "min_value": "1.0", "grace": 240, "reset_relays": false, "relays": ["relay7"]}');
