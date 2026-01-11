# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Note: We are not at a 1.0.0 release. Therefore we do not have a fixed public API
at this moment. Do not rely on this crate as a library as there may be breaking
changes between minor versions.

## [Unreleased]

### Added

- Added an early check to `chithi sync` to ensure datasets do not start with
  leading `/`, a common mistake when invoking `chithi sync`.

### Fixed

- Display of recv options like 'o canmount=noauto' in appeared without the 'o'.
- Recv options like 'o canmount=noauto' were being passed as a single argument
  '-o canmount=noauto'.

## [0.1.0] - 2025-01-04

### Added

- Initial Preview of `chithi sync` command.
