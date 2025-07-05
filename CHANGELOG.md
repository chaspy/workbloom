# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2025-01-05

### Fixed
- Prevent cleanup of newly created branches during `workbloom setup`
- Add check for unmerged commits before removing worktrees
- Exclude the branch being set up from cleanup process

### Changed
- Refactored cleanup function to reduce cognitive complexity
- Improved code organization with smaller, focused functions

### Added
- Unit tests for git module functionality
- `has_unmerged_commits()` method to detect branches with commits not in main

## [0.1.1] - Previous release

### Added
- Initial features

[0.1.2]: https://github.com/chaspy/workbloom/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/chaspy/workbloom/releases/tag/v0.1.1