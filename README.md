# Rocker

Manage your mono-repo binaries locally at ease !

This projects aims at giving you anything you need to easily run your
binaries in a mono-repo setup, giving you an similar experience to what
you may already know with `docker` CLI.

**NOTE** : It does not aim to reproduce the exact same behaviour as what
does `docker`, but stick to its API when relevant.

## Installation

```sh
cargo install --git https://gitlab.com/wykiki/rocker.git rocker

# From sources
git clone https://gitlab.com/wykiki/rocker.git
cargo install --offline --path .
```

## Wipe state

In case you have weird behaviour, you can delete `rocker`'s states located
under `~/.local/state/rocker/`. Doing so won't terminate running subprocesses,
so you may need to terminate them yourself, with some `kill`, like :
`ps u | grep target | awk '{print $2}' | xargs kill`

## TODO

- [x] Reconcile process status at each CLI call
- [ ] Automatically refresh process list when project workspace is updated
- [ ] Refresh process config when `rocker.yaml` is updated
- [ ] Split project into different crates
- [ ] Have an UI
- [ ] Correctly show logs while building
- [ ] Correctly stop child processes

## Potential Naming

- clicker
- pseudocker
- crun
- cr
- rrun
- rr
