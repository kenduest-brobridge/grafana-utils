# completion

## Root

Purpose: generate shell completion scripts from the current `grafana-util` command tree.

When to use:
- when you want Bash or Zsh to complete command names, subcommands, and flags
- when you have installed a new `grafana-util` binary and want completion to match that binary
- when command names or flags changed and your shell completion should be refreshed

Key flags:
- the shell is a required positional value: `bash` or `zsh`
- completion writes the script to stdout; redirect it to the location your shell reads

Examples:

```bash
# generate Bash completion.
grafana-util completion bash
```

```bash
# generate Zsh completion.
grafana-util completion zsh
```

## What this command does

`grafana-util completion` prints a shell completion script generated from the Rust Clap command tree. It does not connect to Grafana, read profiles, or resolve authentication. It only describes the local CLI surface exposed by the binary you are running.

Because the script is generated from the command tree, it should be refreshed after upgrading `grafana-util` or switching to a checkout with different command definitions.

## Install from GitHub with completion

The GitHub install script can install the binary and immediately generate completion from that same binary:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

`auto` detects `bash` or `zsh` from `SHELL`. Use `INSTALL_COMPLETION=bash` or `INSTALL_COMPLETION=zsh` when you want to choose the shell explicitly. Set `COMPLETION_DIR=/path/to/dir` if your shell reads completion files from a different directory.

For an interactive install, pass `--interactive` to the `sh` process after the pipe:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive
```

Interactive mode asks for the binary install directory, whether to install shell completion, and the completion output directory. For Zsh, it can also update `~/.zshrc` with the required `fpath` setup so the completion file is loaded before `compinit`. It reads answers from the terminal, so it still works when the install script itself is coming from `curl`.

For local checkout validation, use the Makefile wrapper instead of waiting for a release artifact:

```bash
make install-local-interactive
```

That target builds the local Rust binary, packs it into a temporary release-style archive, and runs `scripts/install.sh --interactive` against that archive.

## Install for Bash

Choose a completion directory that your Bash setup already loads. A common per-user location is:

```bash
mkdir -p ~/.local/share/bash-completion/completions
grafana-util completion bash > ~/.local/share/bash-completion/completions/grafana-util
```

Start a new shell, or reload your Bash completion setup.

## Install for Zsh

Choose a directory that appears in `fpath`. A common per-user setup is:

```bash
mkdir -p ~/.zfunc
grafana-util completion zsh > ~/.zfunc/_grafana-util
```

Then make sure Zsh loads that directory before `compinit`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit
compinit
```

Put those lines in your Zsh startup file if they are not already present. The interactive installer can add a managed `grafana-util completion` block to `~/.zshrc` for this setup and clear stale `.zcompdump*` completion caches.

## What success looks like

- pressing tab after `grafana-util ` offers root commands such as `dashboard`, `datasource`, `alert`, `access`, `status`, `workspace`, `config`, `version`, and `completion`
- pressing tab after a subcommand offers the flags and nested subcommands known to this binary
- regenerating the script after an upgrade updates the available completions

## Failure checks

- if no completions appear, confirm your shell is loading the file you wrote; for Zsh, `~/.zfunc` must be in `fpath` before `compinit`, and stale `.zcompdump*` files may need to be removed
- if completions are stale, regenerate the script from the currently installed binary
- if Bash or Zsh rejects the file, confirm you used the matching shell value

## Related commands

- [version](./version.md)
- [config](./config.md)
- [status](./status.md)
