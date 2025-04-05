#!/bin/bash
set -e

echo "Building project..."
cargo build

echo ""
echo "===== Azure KeyVault Debug Toolkit ====="
echo "This script will run several tests to diagnose Azure KeyVault connection issues."
echo ""

# Function to separate test sections
function section {
    echo ""
    echo "======================================="
    echo "  $1"
    echo "======================================="
    echo ""
}

# Run the debug script
section "Running credential debug tool"
cargo run --bin debug_azure

# Test integration test
section "Running integration test"
cargo test --test azure_integration_test -- --ignored -v

# Test credential test
section "Running credential test"
cargo test --test azure_credential_test -- --ignored -v

# Test through main application
section "Testing through main application"
cargo run -- --path ~/docker --mode restart-svcs --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secrets.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose

echo ""
echo "===== Debug Complete ====="
echo "Check the output above for any errors or issues."
echo "If any test succeeded, the Azure connection is working in that context."