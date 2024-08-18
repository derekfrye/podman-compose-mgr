# Image Refresh

## Mode: Rebuild
Walks the specified path and refreshes all images contained in `docker-compose.yml` files.

## Example
``` shell
image_refresh --path ~/docker --mode rebuild -e "docker/archive" --build-args USERNAME=`id -un 1000`
```

## Options

### Exclude Path Patterns
Pass in a string, like `docker/archive`, and in `rebuild` mode it'll exclude any `docker-compose.yml` files where the passed in string is contained somewhere within the path. Does *not* use regex.

### Build args
Any strings passed here are passed to `podman build` as `--build-arg`. For example, passing the option on the command-line call to the program <code>--build-args USERNAME=&grave;id -un 1000&grave;</code> will first use your shell to interpret `id -un 1000` and will thus pass something like `--build-arg USERNAME=dfrye` to `podman`.

Notice the argument to this program is `--build-args` whereas it passes each item to `podman build` as `--build-arg arg1`, `--build-arg arg2`, etc.