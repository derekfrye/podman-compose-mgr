# podman-compose-mgr

This program helps manage a directory tree containing `docker-compose.yml` and/or `.container` files. Interactively show image information, and pull, build, or rebuild the images from Dockerfiles/Makefiles.

## Features

-   **TUI Mode**: An optional terminal UI. Manage rebuild jobs from the TUI.
-   **Interactive Image Management**: Interactively pull or (re)build container images.
-   **Regex Search in Rebuild Output**: In job output, `/` or `?` performs a live regex search through `podman build` output. Use `n/N` to jump between matches while the build runs.

## Example

To find all `docker-compose.yml` and `.container` files in your `~/docker` directory, excluding any in a `docker/archive` path, and pass a build argument for the username:

```shell
podman-compose-mgr [--tui] --path ~/docker -e "docker/archive" --build-args "USERNAME=$(id -un)"
```

To skip the interactive prompts entirely and support a oneshot pass, pass `--one-shot` (and optionally `--dry-run` to preview the actions). In that mode every discovered image is built when a Dockerfile or Makefile is present and otherwise pulled from its registry.

For each image found, you will be prompted with the following options:

-   `p`: Pull the image from its upstream registry.
-   `N`: Do nothing and skip to the next image.
-   `d`: Display detailed information about the image, including its creation and pull dates, and whether a local Dockerfile or Makefile exists.
-   `b`: Build the image from a local Dockerfile or Makefile.
-   `s`: Skip all subsequent images with the same name.
-   `?`: Display the help menu.

## Options

-   `-p, --path <PATH>`: The search path for `docker-compose.yml` and `.container` files. Defaults to the current directory.
-   `-v, --verbose`: Increase the verbosity of the output. Use `-vv` for even more detail.
-   `-e, --exclude-path-patterns <PATTERN>`: A regex pattern to exclude paths. Can be specified multiple times.
-   `-i, --include-path-patterns <PATTERN>`: A regex pattern to include paths. If both include and exclude patterns are provided, exclusion is applied first. Can be specified multiple times.
-   `--build-args <ARG>`: A build argument to pass to `podman build` (e.g., `USERNAME=myuser`). Can be specified multiple times.
-   `--temp-file-path <PATH>`: The directory to use for temporary files. Defaults to `/tmp`.
-   `--podman-bin <PATH>`: Override the `podman` executable used for discovery and rebuild commands.
-   `--no-cache`: Pass `--no-cache` to `podman build` to disable the build cache.
-   `--one-shot`: Skip the TUI and CLI prompts and automatically attempt to build every discovered image (falls back to pulling when no Dockerfile/Makefile exists).
-   `--dry-run`: Only valid with `--one-shot`. Print which images would be built or pulled without running the commands.
-   `--tui`: Use the terminal UI mode.
-   `--tui-rebuild-all`: Automatically select and rebuild every discovered image when the TUI opens.

## Why does this exist?

You could do a lot of this with 50 lines of `bash`, `grep`, `curl`, etc. *But*...

1. Bash's `getops` is held back maintaining its hx of portability. For me, arg handling is a big part of an interface, and an interface with built-in help, colored output, typed arguments, etc., is nice.
2. I don't like writing, debugging, or maintaining shell.

## Design documentation

-   [Model-View-Update approach](docs/MVU.md)
