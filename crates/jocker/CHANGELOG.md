# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/Wykiki/jocker/compare/jocker-v0.5.0...jocker-v0.5.1) - 2025-09-17

### Added

- Correctly handle value delimiter for env variables
- Add verbosity levels, stack and process args from env
- May provide stack and processes arguments from env ([#13](https://github.com/Wykiki/jocker/pull/13))
- Add CLI verbosity flag ([#12](https://github.com/Wykiki/jocker/pull/12))
- Replace sqlx by redb ([#8](https://github.com/Wykiki/jocker/pull/8))
- Replace argh by clap ([#5](https://github.com/Wykiki/jocker/pull/5))

### Fixed

- Remove duplicate env_logger invokation

### Other

- Remove unused dependencies ([#4](https://github.com/Wykiki/jocker/pull/4))
- Add per-crate CHANGELOG and README ([#2](https://github.com/Wykiki/jocker/pull/2))
