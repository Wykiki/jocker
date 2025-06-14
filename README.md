# Jocker

Rust mono-repo binaries runner.

This project aims at giving you anything you need to easily run your
binaries in a Rust mono-repo setup, giving you a similar experience to what
you may already know with `docker` CLI.

## Dependencies

`jocker` requires the following external tools to be installed :
- `cargo` (should be included in your Rust toolchain)
- `pueue` (`jocker`'s backend to manage processes)

```sh
# Install Pueue
cargo install pueue@4
# Start Pueue daemon
pueued -d
```

## Installation

### From crates.io

```sh
cargo install jocker
```

### From source

```sh
# Clone
git clone https://github.com/Wykiki/jocker.git

# sqlx setup
cargo install sqlx-cli
cargo sqlx db create
cargo sqlx migrate run --source ${PWD}/crates/jocker-lib/migrations

# Install
cargo install --offline --path crates/jocker
```

## Wipe state

In case you have weird behaviour, you can delete `jocker`'s states located
under `~/.local/state/jocker/` with the command `jocker clean`. Doing so
should also stop and clean related `pueue` tasks. If that's not the case,
you can reset `pueue` tasks with `pueue reset`.

## TODO

- [x] Reconcile process status at each CLI call
- [x] Automatically refresh process list when project workspace is updated
- [x] Refresh process config when `jocker.yaml` is updated
- [x] Handle `stack` keyword in config
- [x] Check references behind `stack` keyword
- [x] Handle `stack.inherits` keyword in config
- [x] Correctly show logs while building
- [x] Split project into different crates
- [x] Correctly show both stdout and stderr of a running process
- [x] Correctly stop child processes
- [x] Command to wipe project state
- [ ] Validate config file with https://docs.rs/jsonschema/latest/jsonschema/ or similar
- [ ] Have an UI
- [ ] When showing all logs, group logs per service before streaming them
