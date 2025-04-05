# Azure KeyVault Integration Tests

This directory contains integration tests for connecting to Azure KeyVault. These tests require real Azure credentials to run.

## Test Files

- `azure_integration_test.rs`: Full integration test that verifies the Azure connection pipeline through the application's code
- `azure_credential_test.rs`: Direct API test for Azure KeyVault credential creation and connection

## Test Data

The tests use real credentials stored in the `personal_testing_data` directory:

- `client_id.txt`: The Azure client ID
- `tenant_id.txt`: The Azure tenant ID
- `secrets.txt`: The Azure client secret
- `vault_name.txt`: The Azure KeyVault name or URL
- `input.json`: Test input for validation
- `outfile.json`: Output file for test results

## Running the Tests

### Test Script

The easiest way to test the Azure connection is to use the provided script:

```bash
./test_azure_connection.sh
```

This script runs the application with the Azure test mode and shows detailed debug output.

### Manual Testing

To manually test the Azure connection:

```bash
cargo run -- --path ~/docker --mode restart-svcs --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secrets.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose
```

### Running Integration Tests

The integration tests are marked with `#[ignore]` to prevent them from running in normal test suites since they require real credentials.

To run the Azure integration test:

```bash
cargo test --test azure_integration_test -- --ignored
```

To run the Azure credential test:

```bash
cargo test --test azure_credential_test -- --ignored
```

## Debugging Azure Authentication Issues

If you encounter authentication issues with Azure, check the following:

1. **Credential Format**: Ensure client IDs and tenant IDs are valid GUIDs without any extra whitespace
2. **Client Secret**: Verify the client secret is correct, without any extra whitespace or newlines
3. **Environment Variables**: The code sets these environment variables for authentication:
   - `AZURE_TENANT_ID`
   - `AZURE_CLIENT_ID`
   - `AZURE_CLIENT_SECRET`
4. **Azure Identity Version**: The application uses Azure Identity version 0.22+, which has a different API than previous versions
5. **KeyVault URL Format**: The URL should be in the format `https://{vault-name}.vault.azure.net`