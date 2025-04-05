#!/bin/bash
set -e

echo "Building project..."
cargo build

echo ""
echo "===== Running Azure connection test ====="
cargo run -- --path ~/docker --mode restart-svcs --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secrets.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose