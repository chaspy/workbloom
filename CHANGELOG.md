# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-08-15

### Added
- Short command aliases for improved usability: `s` for `setup` and `c` for `cleanup`
- Documentation for shell alias `wb` to further reduce typing

### Changed
- Enhanced README with shell integration recommendations for shorter commands

## [0.3.0] - 2025-08-10

### Added
- Execute `.workbloom-setup.sh` script if present in worktree after file copying
- Support for project-specific setup scripts for custom initialization tasks
- Test coverage for setup script detection feature

### Changed
- Setup process now includes an optional setup script execution step

## [0.1.7] - 2025-07-22

### Added
- Support for creating worktrees from remote-only branches (#28)
- Automatic detection and fetching of remote branches in `workbloom setup`
- Branch name validation to prevent command injection attacks
- Enhanced error handling with specific error messages for common failure scenarios

### Changed
- Improved performance by removing unnecessary fetch operations in remote branch detection
- Enhanced error messages in `fetch_remote_branch()` and `create_tracking_branch()` methods

### Security
- Added comprehensive branch name validation to prevent command injection vulnerabilities

## [0.1.6] - 2025-07-21

### Fixed
- Re-apply fix from v0.1.4 that was missing in v0.1.5: Properly identify and protect newly created branches from cleanup
- New branches (like `test1`, `message-crud3`) are no longer incorrectly deleted when running subsequent `workbloom setup` commands

### Changed
- Improved `was_branch_merged_to_main()` to correctly handle branches that point to the same commit as main

## [0.1.5] - 2025-01-19

### Added
- Auto-release workflow that automatically creates tags when version is updated in Cargo.toml

### Fixed
- Fix auto-release workflow awk syntax error for proper CHANGELOG extraction
- Handle existing releases in release workflow to prevent "release already exists" errors
- Prevent build-and-upload jobs from trying to create duplicate releases

### Changed
- Improved CI/CD workflows for more reliable releases

## [0.1.4] - 2025-01-19

### Fixed
- Fix issue where newly created branches with no commits were incorrectly identified as merged branches and deleted
- Implement proper detection of actually merged branches by checking merge commit parents

### Added
- New `was_branch_merged_to_main()` method that checks if a branch was actually merged via a merge commit
- Additional 24-hour safety check for recently created worktrees

### Changed
- Improved branch cleanup logic to distinguish between truly merged branches and new branches that haven't diverged from main

## [0.1.3] - 2025-07-05

### Fixed
- Prevent automatic cleanup of newly created branches during `workbloom setup`
- Filter out branches with no unique commits from merge cleanup to avoid deleting fresh branches

### Changed
- Enhanced cleanup logic to distinguish between truly merged branches and newly created branches

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

[0.1.5]: https://github.com/chaspy/workbloom/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/chaspy/workbloom/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/chaspy/workbloom/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/chaspy/workbloom/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/chaspy/workbloom/releases/tag/v0.1.1