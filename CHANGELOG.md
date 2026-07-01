# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0-alpha.7](https://github.com/p6laris/Void/compare/v0.3.0-alpha.6...v0.3.0-alpha.7) - 2026-06-22

### Added

- *(audio)* embed rich audio notifications via rodio
- *(ui)* add About tab with Ursa Minor art

### Other

- Enhance Stats tab with heatmap filter, tag analytics, and gamified goals. Fixes #37
- Add mouse scrolling support and update help footer hint
- Add scrollable Help tab and update dashboard key hints
- Improved dashboard task handling with Details pane and subtask toggling (Fixes #37)
- Heatmap cell alignment using centered block
- Reset timer mode to Focus on fresh launch (Fixes #40)
- Update footer hint for Zen mode keybindings
- Enhance Zen mode task handling (Fixes #35, Fixes #36)
- Enable auto_advance_task by default
- Improve daily timeline and summary panel graphs

## [0.3.0-alpha.6](https://github.com/p6laris/Void/compare/v0.3.0-alpha.5...v0.3.0-alpha.6) - 2026-06-14

### Other

- release v0.3.0-alpha.5
- release v0.3.0-alpha.4
- gitignore Cursor project config
- Initial commit: Void terminal focus timer

## [0.3.0-alpha.5](https://github.com/p6laris/Void/compare/v0.3.0-alpha.4...v0.3.0-alpha.5) - 2026-06-14

### Other

- release v0.3.0-alpha.4
- gitignore Cursor project config
- Initial commit: Void terminal focus timer

## [0.3.0-alpha.4](https://github.com/p6laris/Void/compare/v0.3.0-alpha.3...v0.3.0-alpha.4) - 2026-06-14

### Other

- gitignore Cursor project config
- Initial commit: Void terminal focus timer

## [0.3.0-alpha.3](https://github.com/p6laris/Void/compare/v0.3.0-alpha.2...v0.3.0-alpha.3) - 2026-06-14

### Fixed

- make Duration import windows-only to avoid unused_import warning on unix
- add missing Duration import for windows build

### Other

- pack the long fucntion param
- deduplicate task selection and settings helpers
- add editorconfig, rust-toolchain, and contributing guide
- split ui/mod.rs into focused sub-modules
- split app.rs into focused sub-modules
- release v0.3.0-alpha.2
- remove chocolatey support and fix rustfmt issues
- remove Chocolatey packaging scripts and configuration
- introduce TaskPayload struct to consolidate task creation and update arguments
- group drawing parameters to resolve too_many_arguments warning
- initial commit

## [0.3.0-alpha.2](https://github.com/p6laris/Void/compare/v0.3.0-alpha.1...v0.3.0-alpha.2) - 2026-06-13

### Fixed

- make Duration import windows-only to avoid unused_import warning on unix
- add missing Duration import for windows build

### Other

- remove chocolatey support and fix rustfmt issues
- remove Chocolatey packaging scripts and configuration
- introduce TaskPayload struct to consolidate task creation and update arguments
- group drawing parameters to resolve too_many_arguments warning
- initial commit
