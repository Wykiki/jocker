# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/Wykiki/jocker/compare/jocker-lib-v0.5.0...jocker-lib-v0.5.1) - 2025-09-17

### Added

- Add verbosity levels, stack and process args from env
- Add CLI verbosity flag ([#12](https://github.com/Wykiki/jocker/pull/12))
- Add retry on pueue connection upon start ([#11](https://github.com/Wykiki/jocker/pull/11))
- Replace sqlx by redb ([#8](https://github.com/Wykiki/jocker/pull/8))

### Other

- Remove unused dependencies ([#4](https://github.com/Wykiki/jocker/pull/4))
- Add per-crate CHANGELOG and README ([#2](https://github.com/Wykiki/jocker/pull/2))

## [0.5.0](https://github.com/Wykiki/jocker/releases/tag/jocker-lib-v0.5.0) - 2025-06-14

### Added

- Control stack with env var JOCKER_STACK
- Replace raw SQLite usage by sqlx
- Start to replace manual process management by pueue
- Put database interactions in its own module
- Add JsonSchema generation, create config module
- Split project into multiple crates

### Fixed

- Change how project version is propagated in cargo workspace

### Other

- Add missing per-crate description and licence
- Revert how sqlx migrate database
- Rework how sqlx access database to make it work in CI
