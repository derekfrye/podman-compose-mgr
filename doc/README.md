# podman-compose-mgr

This program helps manage containers and secrets referenced in `docker-compose.yml` files in a directory tree. 

Features:
- **Regex flexibility** for inclusion and exclusion of paths to process.
- **Azure KeyVault integration** for secure secret management.
- **Container management** for building and updating containers.

By default, the program prompts the user before taking action on any image.

This program has four modes:
1. [Rebuild](#rebuild-mode) (pull or build) select images in `docker-compose.yml` files.
2. [RestartSvcs](#restart-mode) - Restart services within `docker-compose.yml` files.
3. [SecretRefresh](#secret-refresh-mode) - Refreshes and updates secrets in Azure KeyVault.
4. [SecretRetrieve](#secret-retrieve-mode) - Retrieves and validates secrets from Azure KeyVault.

## Examples

### Pull or build images, including and excluding a pattern
``` shell
podman-compose-mgr --path ~/docker --mode rebuild -e "docker/archive" --build-args USERNAME=`id -un 1000`
```
- Recursively pull or build (prompting the user each time) images defined in `docker-compose.yml` files under subdirectories of `~/docker`,
- skip any `docker-compose.yml` files that match `docker/archive` anywhere within their path,
- passing `--build-arg` to `podman`, after your shell evaluates `id -un 1000`.
- **Note:** exclusion takes precedence over inclusion

### Retrieve and validate secrets from Azure KeyVault
```bash
podman-compose-mgr --path ~/docker --mode secret-retrieve --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secrets.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose
```

## Rebuild Mode
Walks the specified path and refreshes all images contained in `docker-compose.yml` files.

## Restart Mode
Restarts services defined in `docker-compose.yml` files. This mode can also be used for testing Azure KeyVault connections.

## Secret Refresh Mode
Finds `.env` files, uploads them to Azure Key Vault, and generates a JSON record.

## Secret Retrieve Mode
Validates secrets stored in Azure KeyVault against local files.

## Azure KeyVault Integration

This project includes integration with Azure KeyVault for storing and retrieving secrets. The integration has been updated to work with Azure Identity v0.22.

### Testing Azure KeyVault Connection

Several tools are provided for testing the Azure connection:

1. **Debug Script**: Run the debug script to diagnose Azure KeyVault connection issues:

```bash
./debug_azure_all.sh
```

2. **Integration Tests**: Run the integration tests to verify Azure KeyVault connection:

```bash
cargo test --test azure_integration_test -- --ignored -v
cargo test --test azure_credential_test -- --ignored -v
```

3. **Application Testing**: Test the Azure connection through the main application:

```bash
cargo run -- --path ~/docker --mode restart-svcs --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secrets.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose
```

### Required Test Files

For testing Azure KeyVault integration, provide the following files:

- `tests/personal_testing_data/client_id.txt`: Azure client ID
- `tests/personal_testing_data/tenant_id.txt`: Azure tenant ID
- `tests/personal_testing_data/secrets.txt`: Azure client secret
- `tests/personal_testing_data/vault_name.txt`: Azure KeyVault name
- `tests/personal_testing_data/input.json`: Test input for validation
- `tests/personal_testing_data/outfile.json`: Output file for test results

## Troubleshooting

If you encounter authentication issues with Azure:

1. **Credential Format**: Ensure client IDs and tenant IDs are valid GUIDs without whitespace
2. **Client Secret**: Verify the client secret is correct without extra whitespace or newlines
3. **Vault Name**: If using a vault URL, make sure it's properly formatted as `https://{vault-name}.vault.azure.net`
4. **Run Debug Tools**: Use the provided debug tools to diagnose connection issues

## Options

### Exclude Path Patterns
Passing in a string, like `docker/archive`, and in `rebuild` mode it'll exclude any `docker-compose.yml` files it finds where the passed string matches within the path. Simple text match, *not* a regex.

### Build args
Strings passed here are passed to `podman build` as `--build-arg`. For example, passing the option <code>--build-args USERNAME=&grave;id -un 1000&grave;</code> will use your shell to interpret `id -un 1000` and pass `--build-arg USERNAME=(whatever your username is)` to `podman` during build.

You can pass multiple build options to `podman` like so:
``` shell
podman-compose-mgr --build-args USERNAME=`id -un 1000` --build-args VERSION=1.2.3
```

### Azure KeyVault Options

- `--secrets-client-id`: Azure client ID or path to file containing it
- `--secrets-client-secret-path`: Path to file containing the Azure client secret
- `--secrets-tenant-id`: Azure tenant ID or path to file containing it
- `--secrets-vault-name`: Azure KeyVault name or path to file containing it
- `--secret-mode-output-json`: Path to write output JSON results
- `--secret-mode-input-json`: Path to read input JSON for validation

## Why does this exist?

### Can't this be just 50 lines of bash?

Yes, you could do a lot of this with 50 lines of `bash`, `grep`, `tput`. *But*...

1. Bash's `getops` is held back maintaing its hx of portability. For me, arg handling is a big part of an interface, and an interface with built-in help, colored output, typed arguments, etc., is tablestakes for me[^1].
2. I don't like writing, debugging, or maintaining shell after about 30 lines.

### Isn't podman-compose a bad way to run containers?

It might be a dead-end eventually ([1](https://github.com/containers/podman-compose/issues/276), [2](https://github.com/containers/podman-compose/issues/629)). And some features don't work reliably, that I wish would ([1](https://github.com/containers/podman-compose/issues/715)).

But, `podman` and `podman-compose` are working for my servers, and `docker-compose` appears to be a first-class citizen in Docker ecosystem. And, `minikube` seems too complex for my use case (building images locally and running rootless).

[^1]: I'm old and I use my stuff for years, so if I don't have good, built-in help and self-explanatory command line parameters, I find I have to re-read the source to learn the right incantation of command line params in my setup. I've found [clap](https://docs.rs/clap/latest/clap/) is a good balance, and keeps me out of re-reading source code for most times I need to change the params I'm passing a program, so I've grown to use it when building something new.