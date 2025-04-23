# Tests Documentation for podman-compose-mgr

## Test Files and Directories

### test1_walk_dirs_varying_term_size.rs

Tests directory traversal with different terminal sizes, making sure the program correctly clips/shortens user prompts based on terminal width.

**Files used:**
- `tests/test1/image1/docker-compose.yml` and `tests/test1/image2/docker-compose.yml`: Test docker-compose files used for traversal testing
- `.vscode/launch.json`: Used to extract test configuration (the vars in this config section drive the arguments to this test)

This test verifies that `walk_dirs` function traverses directories containing docker-compose files. Uses different terminal display widths (40 and 60 characters), making sure the output is correctly limited to the width of the terminal (inclusive of potential user input).

### test2_azure_client.rs

Integration test for Azure Key Vault connection (currently) using azure_identity v0.21. (Version 0.22, the latest as of this, seems to only support environment vars for auth, which seem like an anti-pattern to me.)

**Files used:**
- `tests/personal_testing_data/client_id.txt`: Must contain an `appId` for connecting to Azure Key Vault (test cannot run w/o it)
- `tests/personal_testing_data/tenant_id.txt`: Azure Key Vault `tenant`
- `tests/personal_testing_data/vault_name.txt`: Azure Key Vault vault name
- `tests/personal_testing_data/secret.txt`: Azure Key Vault `password`

This test requires real credentials. It validates that the Azure SDK integration retrieves secrets from Azure Key Vault.

### test3_secret_init.rs

Tests the secret initialization process.

**Files used:**
- `tests/test3_and_test4/test_input.json`: Contains input for initializing secrets
- `tests/test3_and_test4/test_input_run2.json`: Contains input for second run with an additional file
- `tests/test3_and_test4/a`, `b`, `c`, `d d`, `e e`, `f`: Test files of varying types and sizes
  - `a`, `b`, `c`: Small text files
  - `d d`: Text file with spaces in name
  - `e e`: Binary file that requires base64 encoding
  - `f`: Additional file for second run test

Tests the `initialize::process` function that scans files, calculates hashes, determines encodings (base64 vs utf8), and creates JSON entries for cloud storage. Second validates that all 6 entries are present in output (files a - "e e" are updated, and 1 new (file f)).

### test4_secret_upload.rs

Tests the secret upload process with varying terminal sizes.

**Files used:**
- Same files as test3: `tests/test3_and_test4/a`, `b`, `c`, `d d`

Tests the upload user workflow using mock clients for Azure KeyVault, B2, and R2 storage. Verifies prompt formatting at different terminal widths, tests user interactions for viewing details and approving uploads.

### test6_r2_upload.rs

Tests the Cloudflare R2 storage upload process.

**Files used:**
- Same files as test3: `tests/test3_and_test4/a`, `b`, `c`, `d d`

Tests R2 storage upload functionality, including file existence checks and size comparison between local files and those already in R2 storage. Tests showing file details before upload and proper JSON output generation.

### test7_new_init_fmt.rs

Tests the new cloud section parsing functionality in the initialization process.

**Files used:**
- `tests/test7_new_init_fmt/test_for_init`: Input file format for initialization
- `tests/test7_new_init_fmt/test_init_json_format.json`: Expected JSON format after parsing (production creates this from a flat file, so we test that)
- `tests/test7_new_init_fmt/test_output.json`: Output from initialization process

Tests that the `check_init_filepath` function correctly parses cloud provider configurations from input files and creates appropriate JSON output.

### test8_base64.rs

Tests file encoding detection and base64 encoding/decoding.

**Files used:**
- `tests/test3_and_test4/a`: UTF-8 text file
- `tests/test3_and_test4/e e`: Binary file
- `tests/test3_and_test4/e e.base64`: Base64-encoded version of the `e e` file

Validates that binary files are correctly detected and base64-encoded (using production fn calls), while text files remain as UTF-8. Also tests that decoding base64 content produces identical content to the original file.

### test9_output_entries.rs

Tests that output entries are correctly generated for different cloud providers.

**Files used:**
- `tests/test7_new_init_fmt/test_output.json`: Input for generating output entries
- `tests/test9/reference_output.json`: Reference output for validation
- `tests/test9/generated_output.json`: Generated output during test (this is not stored in git)

Validates that the output entries created for different cloud providers (Azure KeyVault, B2, R2) have the correct structure and fields.

### test10_no_base64_r2_and_az.rs

Tests that base64 encoding is only applied for files destined for Azure KeyVault, not for R2 or B2 storage.

**Files used:**
- `tests/test3_and_test4/a`: UTF-8 text file
- `tests/test3_and_test4/e e`: Binary file
- `tests/test3_and_test4/e e.base64`: Base64-encoded version of the `e e` file

Confirms that binary files are only base64-encoded when destined for Azure KeyVault, not for R2 or B2 storage. Also validates the file size threshold for determining which cloud provider to use.
