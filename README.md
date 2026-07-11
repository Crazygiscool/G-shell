# G-Shell

A POSIX-compliant shell written in Rust — built from scratch as part of the
[CodeCrafters "Build Your Own Shell"](https://app.codecrafters.io/courses/shell/overview) challenge.

## Features

- **Builtins**: `echo`, `pwd`, `cd`, `type`, `exit`, `history`, `export`, `unset`, `set`, `env`, `test`/`[`, `alias`/`unalias`, `source`, `help`
- **Pipes & redirects**: `|`, `>`, `>>`, `<`, `2>`, `2>>`, `2>&1`, heredocs (`<<`)
- **Scripting**: `if`/`elif`/`else`/`fi`, `for`/`do`/`done`, `while`/`do`/`done`, `case`/`in`/`esac`, subshells `( )`
- **Operators**: `&&`, `||`, `;`, `&` (background), `!` (pipeline negation)
- **Expansion**: `$VAR`, `${VAR}`, `$?`, `$(cmd)`, `` `cmd` ``, `~`, globs (`*?[a-z]`), history expansion (`!!`, `!$`, `!N`)
- **Tab completion**: builtins + PATH lookup + filesystem
- **Custom theme system**:
  - `GS_PROMPT_FORMAT` — segment-based prompt (`{user}`, `{host}`, `{path}`, `{git}`, `{exit}`, etc.)
  - `GS_OH_MY_POSH_THEME` — load oh-my-posh `.omp.json` themes
  - `GS_STYLE_<SEGMENT>` — per-segment colors (`"cyan bold"`, `"white on_blue"`, hex `"#ff0000"`)
  - Legacy `PS1` backslash escapes still supported (`\w`, `\u`, `\h`, `\$`, `\t`)
- **REPL**: multi-line input, history save/load, `HISTFILE`, `!` history expansion, Ctrl+C handling
- **Init file**: `~/.gshellrc` or `$GSHELLRC` sourced on startup

## Install

### Prerequisites

- Rust 1.91+ (`rustup install 1.91 && rustup default 1.91`)

### Arch Linux (AUR)

```sh
yay -S g-shell
# or: paru -S g-shell
```

### Via install.sh

```sh
git clone https://github.com/Crazygiscool/G-shell
cd G-shell
chmod +x install.sh
./install.sh
```

Installs to `~/.local/bin/g-shell` (or a custom path: `./install.sh /usr/local/bin`).

### Via cargo

```sh
cargo install --path .
```

### From source (no install)

```sh
./gshell.sh           # build debug + run
./gshell.sh --bin     # build release + run
```

## Quick start

```sh
g-shell
```

Or customize the prompt in `~/.gshellrc`:

```sh
export GS_PROMPT_FORMAT='{user}@{host}:{path}{git}{exit} {prompt} '
export GS_STYLE_PATH="cyan bold"
export GS_STYLE_GIT="yellow"
```

Oh-my-posh themes:

```sh
export GS_OH_MY_POSH_THEME="$HOME/.poshthemes/catppuccin.omp.json"
```

---

## CodeCrafters

This project originated from the
[CodeCrafters "Build Your Own Shell"](https://app.codecrafters.io/courses/shell/overview) challenge.

### Passing the first stage

```sh
git commit -am "pass 1st stage" # any msg
git push origin master
```

### Stage 2 & beyond

1. Ensure you have `cargo (1.91)` installed locally
1. Run `./your_program.sh` to run your program
1. Commit your changes and run `git push origin master` to submit your solution
