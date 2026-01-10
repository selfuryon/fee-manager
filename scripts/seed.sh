#!/usr/bin/env bash
# Seed test data into the fee-manager database
# Usage: ./scripts/seed.sh [DATABASE_URL]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATABASE_URL="${1:-${DATABASE_URL:-postgres://feemanager:feemanager@localhost/feemanager}}"

echo "Seeding database: $DATABASE_URL"
echo ""

psql "$DATABASE_URL" -f "$SCRIPT_DIR/seed_testdata.sql"

echo ""
echo "Done!"
