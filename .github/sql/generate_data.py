import psycopg2
import random
import json
import os
from faker import Faker
from urllib.parse import urlparse

# Getting the database connection string from the environment variable
DATABASE_URL = os.getenv("DATABASE_URL")

# Parsing the DATABASE_URL to extract connection details
url = urlparse(DATABASE_URL)

# Extract connection parameters
DB_HOST = url.hostname
DB_PORT = url.port
DB_NAME = url.path[1:]  # removing the first '/' from the path
DB_USER = url.username
DB_PASSWORD = url.password

# Initialize Faker for generating random data
fake = Faker()

# Function to generate a random Ethereum address
def generate_eth_address():
    return f"0x{fake.hexify(text='^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^', upper=True)}"

# Function to generate a default config
def generate_default_config():
    return {
        "fee_recipient": generate_eth_address(),
        "gas_limit": random.randint(1000000, 10000000),
        "min_value": round(random.random(), 2),
        "grace": random.randint(500, 1000),
        "relays": {}
    }

# Function to generate a proposer config with a reference to default configs
def generate_proposer_with_default(default_configs):
    return {
        "fee_recipient": generate_eth_address(),
        "gas_limit": random.randint(200000, 500000),
        "min_value": round(random.random(), 2),
        "grace": random.randint(500, 1000),
        "reset_relays": random.choice([True, False]),
        "relays": {},
        "default_configs": random.sample(default_configs, 2)  # binding to 2 random default configs
    }

# Function to generate a proposer config without reference to default configs
def generate_proposer():
    return {
        "fee_recipient": generate_eth_address(),
        "gas_limit": random.randint(200000, 500000),
        "min_value": round(random.random(), 2),
        "grace": random.randint(500, 1000),
        "relays": {}
    }

# Function to establish a connection to the PostgreSQL database
def connect_to_db():
    return psycopg2.connect(
        host=DB_HOST,
        port=DB_PORT,
        dbname=DB_NAME,
        user=DB_USER,
        password=DB_PASSWORD
    )

# Function to insert a config into the execution_configs table
def insert_config(cursor, config_id, config_type, config, default_configs=None):
    config_json = json.dumps(config)
    if default_configs:
        # Convert the default_configs list to a PostgreSQL array format
        default_configs_str = "{" + ",".join([f"'{item}'" for item in default_configs]) + "}"
        cursor.execute("""
            INSERT INTO execution_configs (config_id, config_type, default_configs, config)
            VALUES (%s, %s, %s, %s)
        """, (config_id, config_type, default_configs_str, config_json))
    else:
        cursor.execute("""
            INSERT INTO execution_configs (config_id, config_type, config)
            VALUES (%s, %s, %s)
        """, (config_id, config_type, config_json))

# Main function to generate and insert test data into the database
def generate_and_insert_data():
    # Connect to the database
    conn = connect_to_db()
    cursor = conn.cursor()

    # Generate 10 default configs
    default_configs = []
    for i in range(1, 11):
        config = generate_default_config()
        insert_config(cursor, f"default{i}", "default", config)
        default_configs.append(f"default{i}")
    
    # Generate 20 proposer configs with references to default configs
    for i in range(1, 21):
        config = generate_proposer_with_default(default_configs)
        insert_config(cursor, f"proposer{i}", "proposer", config, default_configs)
    
    # Generate the remaining proposer configs without references
    for i in range(21, 100001):
        config = generate_proposer()
        insert_config(cursor, f"proposer{i}", "proposer", config)
    
    # Commit the changes to the database
    conn.commit()

    # Close the database connection
    cursor.close()
    conn.close()

    print("Test data insertion complete.")

if __name__ == "__main__":
    # Run the data generation and insertion
    generate_and_insert_data()

