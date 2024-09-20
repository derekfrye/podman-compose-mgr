# podman-compose-mgr

This program helps me manage podman containers, defined in `docker-compose.yml` files, recursively under a directory. I built this to have cli regex pattern flexiblity to selectively start/stop `podman-compose`-managed containers, pulling or building updating images, and (eventually) managing secrets.

By default, the program prompts the user before taking action on any image.

This program has three modes:
1. [Rebuild](#rebuild-mode) (pull or build) select images in `docker-compose.yml` files.
2. [Start/stop] services within `docker-compose.yml` files.
3. Secret management (which isn't implmented yet)

## Examples

### Refresh (pull or build) all images, excluding a pattern
``` shell
podman-compose-mgr --path ~/docker --mode rebuild -e "docker/archive" --build-args USERNAME=`id -un 1000`
```
- Recursively pull or build (prompting the user each time) for all images defined in `docker-compose.yml` files under subdirectories of `~/docker`,
- skipping any `docker-compose.yml` files that match `docker/archive` anywhere within their path,
- passing `--build-arg` to `podman`, after your shell evaluates `id -un 1000`.

## Rebuild mode
Walks the specified path and refreshes all images contained in `docker-compose.yml` files.

## Start/stop mode

## Options

### Exclude Path Patterns
Passing in a string, like `docker/archive`, and in `rebuild` mode it'll exclude any `docker-compose.yml` files it finds where the passed string matches within the path. Simple text match, *not* a regex.

### Build args
Strings passed here are passed to `podman build` as `--build-arg`. For example, passing the option <code>--build-args USERNAME=&grave;id -un 1000&grave;</code> will use your shell to interpret `id -un 1000` and pass `--build-arg USERNAME=(whatever your username is)` to `podman` during build.

You can pass multiple build options to `podman` like so:
``` shell
podman-compose-mgr --build-args USERNAME=`id -un 1000` --build-args VERSION=1.2.3
```

[^1]: I'm old and I use my stuff for years, so if I don't have good, built-in help and self-explanatory command line parameters, I find I have to re-read the source to learn the right incantation of command line params in my setup. I've found [clap](https://docs.rs/clap/latest/clap/) is a good balance, and keeps me out of re-reading source code for most times I need to change the params I'm passing a program, so I've grown to use it when building something new.

## Why does this exist?

### Can't this be just 50 lines of bash?

Yes, you could do a lot of this with 50 lines of `bash`, `grep`, `tput`. *But*...

1. Bash's `getops` is held back maintaing its hx of portability. For me, arg handling is a big part of an interface, and an interface with built-in help, colored output, typed arguments, etc., is tablestakes for me[^1].
2. I don't like writing, debugging, or maintaining shell after about 30 lines.

### Isn't podman-compose a bad way to run containers?

It might be a dead-end eventually ([1](https://github.com/containers/podman-compose/issues/276), [2](https://github.com/containers/podman-compose/issues/629)). And some features don't work reliably, that I wish would ([1](https://github.com/containers/podman-compose/issues/715)).

But, `podman` and `podman-compose` are working for my servers, and `docker-compose` appears to be a first-class citizen in Docker ecosystem. And, `minikube` seems too complex for my use case (building images locally and running rootless).