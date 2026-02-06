# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-06

### Changed
- Bump MSRV from 1.65.0 to 1.78.0
- Update dependencies (tokio 1.41.1 → 1.49.0, and others)

### Added
- CI release automation with workspace publish and dry-run workflows
- MSRV verification script

### Fixed
- Publish workflow now checks out the release tag instead of main branch

## [0.1.0]

### Added
- Initial release with metered tokio channels
- Prometheus metrics integration
- MPSC, Broadcast, and Watch channel implementations
- Comprehensive test coverage
- Tracing integration with debug-level logging
