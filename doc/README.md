# podman-compose-mgr

This program helps manage a directory tree containing docker-compose files, including the containers and secrets referenced in each. This is an interactive cli. By default, we prompt the user before taking an action.

Features:
- **Container management** for building and updating containers.
- **Azure Key Vault integration** for secure secret management.
- **Regex flexibility** for inclusion and exclusion of paths to process.

This program has three *modes*:
1. [Rebuild](#rebuild-mode) (pull or build) images in `docker-compose.yml` files.
2. (Not yet fully tested) [SecretRetrieve](#secret-retrieve-mode) - Retrieves and validate on-disk secrets against secrets stored in Azure Key Vault.
3. (Not yet implemented or tested) [SecretRefresh](#secret-refresh-mode) - Refreshes and updates secrets in Azure KeyVault.

## Examples

### Pull or build images, including and excluding a pattern
``` shell
podman-compose-mgr --path ~/docker --mode rebuild -e "docker/archive" --build-args USERNAME=`id -un 1000`
```
- `--mode` to specify we're in build/rebuild mode
- `--path` to recursively pull or build (prompting the user each time) all images it finds in all `docker-compose.yml` files under subdirectories of `~/docker`,
- optional `-e` to skip any `docker-compose.yml` files that match `docker/archive` anywhere within their path,
    - **Note:** exclusion takes precedence over inclusion
- optional `--build-arg` will pass this value *after shell evaluation* of an expression (`id -un 1000` in this example) to `podman`.

### Retrieve and validate secrets from Azure Key Vault
```shell
podman-compose-mgr --path ~/docker --mode secret-retrieve --secrets-client-id tests/personal_testing_data/client_id.txt --secrets-client-secret-path tests/personal_testing_data/secret.txt --secrets-tenant-id tests/personal_testing_data/tenant_id.txt --secrets-vault-name tests/personal_testing_data/vault_name.txt --secret-mode-output-json tests/personal_testing_data/outfile.json --secret-mode-input-json tests/personal_testing_data/input.json --verbose
```
- `--mode` to specify we're in validation of on-disk secrets
- several switches to tell the program where to read in sensitive values (Azure client ID, secret, tenant, and vault name)
- `--secret-mode-input-json` to specify the json containing the list of on-disk secrets to validate against Azure Key Vault (TODO: document this file format)
- `--secret-mode-output-json` to... (TODO: is this needed?)

For more details on this mode, see [secrets retrieve documentation](secrets_retrieve.md). 

## Options

### Build args
Strings passed here are passed to `podman build` as `--build-arg`. For example, passing the option <code>--build-args USERNAME=&grave;id -un 1000&grave;</code> will use your shell to interpret `id -un 1000` and pass `--build-arg USERNAME=(whatever your username is)` to `podman` during build.

You can pass multiple build options to `podman` like so:
``` shell
podman-compose-mgr --build-args USERNAME=`id -un 1000` --build-args VERSION=1.2.3
```

## Why does this exist?

### Can't this be just 50 lines of shell?

Yes, you could do a lot of this with 50 lines of `bash`, `grep`, `tput`, `curl`. *But*...

1. Bash's `getops` is held back maintaining its hx of portability. For me, arg handling is a big part of an interface, and an interface with built-in help, colored output, typed arguments, etc., is nice[^1].
2. I don't like writing, debugging, or maintaining shell after about 30 lines.

### Isn't podman-compose bad/unsupported?

It might be a dead-end eventually ([1](https://github.com/containers/podman-compose/issues/276), [2](https://github.com/containers/podman-compose/issues/629)). And some features don't work reliably, that I wish would ([1](https://github.com/containers/podman-compose/issues/715)).

But, `podman` and `podman-compose` are working for my servers, and `docker-compose` appears to be a first-class citizen in Docker ecosystem. And, `minikube` seems too complex for my use case.

[^1]: I use my stuff for years; if I don't have good, built-in help and self-explanatory command line parameters, I have to re-read source to learn the right mix of params in my setup, which sucks.