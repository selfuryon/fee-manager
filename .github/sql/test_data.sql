-- Clear existing data if needed
DELETE FROM execution_configs;

-- Insert default configurations
INSERT INTO execution_configs 
(config_id, config_type, config) 
VALUES 
('lido', 'default', '{
  "version": 2,
  "fee_recipient": "0x8b7269a86c3b673822ebf076c8322bba62917b83",
  "gas_limit": "30000000",
  "min_value": "0.1",
  "grace": 0,
  "relays": {
    "https://relay.mainnet.ethpandaops.io": {
      "public_key": "0x86ee4d8f3bc69671505f7bd1056ab6b07322e51ccc10b839f64aa580752f26a022ffcd3c97e9e5d2a2a7086c4f9f0ce6"
    },
    "https://boost-relay.flashbots.net": {
      "public_key": "0xa1d1ad0714035353258038e964ae9675dc0252ee22cea896825c01458e1807bfad2f9969338798548d9858a571f7425c",
      "min_value": "0.15"
    },
    "https://bloxroute.max-profit.blxrbdn.com": {
      "public_key": "0x8b5d4e3b0a69c29c5755c630bdc897c2b90584c7c2df9a6683bbe0867ca21464c43f940c26b4fa58c72745d4b0ac6b2b",
      "fee_recipient": "0x6a192a82372525e721c2902921cdb3daca86fe8c",
      "gas_limit": "28000000"
    }
  }
}');

INSERT INTO execution_configs 
(config_id, config_type, config) 
VALUES 
('default', 'default', '{
  "version": 2,
  "fee_recipient": "0xc1e92bd5f1d2584c84c725b1d9039cc88b896504",
  "gas_limit": "25000000",
  "min_value": "0.2",
  "grace": 3,
  "relays": {
    "https://relay.mainnet.ethpandaops.io": {
      "public_key": "0x86ee4d8f3bc69671505f7bd1056ab6b07322e51ccc10b839f64aa580752f26a022ffcd3c97e9e5d2a2a7086c4f9f0ce6"
    },
    "https://boost-relay.flashbots.net": {
      "public_key": "0xa1d1ad0714035353258038e964ae9675dc0252ee22cea896825c01458e1807bfad2f9969338798548d9858a571f7425c",
      "min_value": "0.25"
    }
  }
}');

INSERT INTO execution_configs 
(config_id, config_type, config) 
VALUES 
('default-aggressive', 'default', '{
  "version": 2,
  "fee_recipient": "0x94750381be1aba0504c666ee1db118f68f0780d4",
  "gas_limit": "35000000",
  "min_value": "0.05",
  "grace": 1,
  "relays": {
    "https://relay.mainnet.ethpandaops.io": {
      "public_key": "0x86ee4d8f3bc69671505f7bd1056ab6b07322e51ccc10b839f64aa580752f26a022ffcd3c97e9e5d2a2a7086c4f9f0ce6"
    },
    "https://boost-relay.flashbots.net": {
      "public_key": "0xa1d1ad0714035353258038e964ae9675dc0252ee22cea896825c01458e1807bfad2f9969338798548d9858a571f7425c"
    },
    "https://bloxroute.max-profit.blxrbdn.com": {
      "public_key": "0x8b5d4e3b0a69c29c5755c630bdc897c2b90584c7c2df9a6683bbe0867ca21464c43f940c26b4fa58c72745d4b0ac6b2b"
    },
    "https://agnostic-relay.net": {
      "public_key": "0xa7ab7a996c8584251c8f925da3170bdfd6ebc75d50f5ddc4050a6fdc77f2a3b5cd5dbd89dee0e1ec1a5cc929c1d9dea6"
    },
    "https://aestus.live": {
      "public_key": "0xa28b6cf3dd5a8cad8dfa79e66b0dd70240101312c2757d822db53708e4c4ff68c370966a0e1ca532cd136fea47813c2c"
    }
  }
}');

-- Insert proposer configurations
INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('0x8021a28e9cf0463daa1f4c935f7d9983c862c3c7c92bd6d4891fa8c3788bbe3425c4f4c01b1ae8391a5d57a52ae093ea', 'proposer', 
NULL,
'{
  "fee_recipient": "0x1c675ed17ea063dc83ccf444da8d7da36f3d0a1c",
  "min_value": "0.15",
  "relays": {
    "https://relay.mainnet.ethpandaops.io": {
      "min_value": "0.2"
    },
    "https://boost-relay.flashbots.net": {
      "min_value": "0.25"
    }
  }
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('0x8c2731e8f6c2797c4ec7c51c8e6649d2107c077c05aae74e9c67467efa3b2190c42b427ea0fec651383bb25cc8baed21', 'proposer', 
NULL,
'{
  "reset_relays": true,
  "fee_recipient": "0x3214dc7c1d41c470f35cc842d851225c1729698e",
  "relays": {
    "https://boost-relay.flashbots.net": {
      "public_key": "0xa1d1ad0714035353258038e964ae9675dc0252ee22cea896825c01458e1807bfad2f9969338798548d9858a571f7425c",
      "min_value": "0.15"
    }
  }
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('0x90c7416266513073b4f25973ed839aa5d1dc6a9cafe2a9a7acaf3f2b7c4b54bd1dc2f7e434d78fed0c889641343fd4a2', 'proposer', 
NULL,
'{
  "gas_limit": "28000000"
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('^Wallet 1/.*$', 'proposer', 
ARRAY['default'], 
'{
  "fee_recipient": "0xd851d0fceacdc3cf1ae8b9b6e45a0a9f58e7f558"
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('^Wallet 1/Account 2$', 'proposer', 
ARRAY['default'], 
'{
  "fee_recipient": "0xe0b6a3bc4908c8cd82121d0a3543ede6cd9b06bd",
  "min_value": "0.3"
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('^Wallet 2/Account [123]$', 'proposer', 
ARRAY['default-aggressive'], 
'{
  "min_value": "0.08",
  "relays": {
    "https://boost-relay.flashbots.net": {
      "min_value": "0.1"
    }
  }
}');

INSERT INTO execution_configs 
(config_id, config_type, default_configs, config) 
VALUES 
('^Wallet 2/Account 4$', 'proposer', 
ARRAY['default-aggressive', 'lido'], 
'{
  "reset_relays": true
}');
