# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
