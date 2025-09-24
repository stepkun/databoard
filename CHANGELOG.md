# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html),
especially the [Rust flavour](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Schema] - 2025-??-??

### Added

### Changed

### Fixed

### Removed

## [0.2.1] - 2025-09-24

### Added
- `Debug` implementation for `Databoard`

## [0.2.0] - 2025-09-22

### Changed
- hide inner structure, renamed `DataboardPtr` to `Databoard` 

### Fixed
- minimum Rust version set to 1.88.0

## [0.1.1] - 2025-09-19

### Added
- `try_get_ref(...)` & `try_get_mut_ref(...)` methods
- public visibility of `EntryReadGuard` & `EntryWriteGuard`

### Removed
- need for `T`to implement `Clone` for all methods but `get()`

## [0.1.0] - 2025-09-18

Version 0.1.0 is a fundamentally working implementation of the hierarchical databoard.
To be changed in future versions:
- The need of `T` to implement `Clone`.
- The implementation of `Remappings`, which is very simple right now.