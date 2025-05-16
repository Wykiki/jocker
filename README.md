# Jocker

Manage your mono-repo binaries locally at ease !

This projects aims at giving you anything you need to easily run your
binaries in a mono-repo setup, giving you an similar experience to what
you may already know with `docker` CLI.

**NOTE** : It does not aim to reproduce the exact same behaviour as what
does `docker`, but stick to its API when relevant.

## Installation

```sh
cargo install --git https://gitlab.com/wykiki/jocker.git jocker

# From sources
git clone https://gitlab.com/wykiki/jocker.git
cargo install --offline --path .
```

## Wipe state

In case you have weird behaviour, you can delete `jocker`'s states located
under `~/.local/state/jocker/`. Doing so won't terminate running subprocesses,
so you may need to terminate them yourself, with some `kill`, like :
`ps u | grep target | awk '{print $2}' | xargs kill`

## TODO

- [x] Reconcile process status at each CLI call
- [x] Automatically refresh process list when project workspace is updated
- [x] Refresh process config when `jocker.yaml` is updated
- [x] Handle `stack` keyword in config
- [x] Check references behind `stack` keyword
- [x] Handle `stack.inherits` keyword in config
- [x] Correctly show logs while building
- [ ] Split project into different crates
- [ ] Have an UI
- [ ] Correctly show both stdout and stderr of a running process
- [ ] When showing all logs, group logs per service before streaming them
- [ ] Correctly stop child processes
- [ ] Command to wipe project state
- [ ] Validate config file with https://docs.rs/jsonschema/latest/jsonschema/ or similar

