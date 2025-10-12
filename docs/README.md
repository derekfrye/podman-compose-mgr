# podman-compose-mgr

This program helps manage a directory tree containing `docker-compose.yml` and `.container` files. It recursively searches for these files and interactively prompts the user to either pull the upstream image or build it from a local Dockerfile/Makefile.

## Features

-   **Interactive Image Management**: Interactively choose to pull or build container images.
-   **Flexible Path Filtering**: Use regex patterns to include or exclude specific paths from processing.
-   **Custom Build Arguments**: Pass build arguments to `podman build`.
-   **TUI Mode**: An optional terminal UI for a more visual experience.
-   **Queue Rebuilds**: Manage rebuild jobs from the TUI, including an option to auto-queue everything on launch.

## Documentation

-   [Model-View-Update approach](docs/MVU.md)

## Example

To find all `docker-compose.yml` and `.container` files in your `~/docker` directory, excluding any in a `docker/archive` path, and pass a build argument for the username:

```shell
podman-compose-mgr --path ~/docker -e "docker/archive" --build-args "USERNAME=$(id -un)"
```

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
-   `--tui`: Use the terminal UI mode.
-   `--tui-rebuild-all`: Automatically select and rebuild every discovered image when the TUI opens.

## Why does this exist?

You could do a lot of this with 50 lines of `bash`, `grep`, `tput`, `curl`. *But*...

1. Bash's `getops` is held back maintaining its hx of portability. For me, arg handling is a big part of an interface, and an interface with built-in help, colored output, typed arguments, etc., is nice[^1].
2. I don't like writing, debugging, or maintaining shell after about 30 lines.
