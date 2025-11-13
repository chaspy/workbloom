# Workbloom

[![Build](https://github.com/chaspy/workbloom/actions/workflows/build.yml/badge.svg)](https://github.com/chaspy/workbloom/actions/workflows/build.yml)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=chaspy_workbloom&metric=alert_status)](https://sonarcloud.io/dashboard?id=chaspy_workbloom)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=chaspy_workbloom&metric=coverage)](https://sonarcloud.io/dashboard?id=chaspy_workbloom)
[![Crates.io](https://img.shields.io/crates/v/workbloom.svg)](https://crates.io/crates/workbloom)

A Git worktree management tool written in Rust that automates worktree setup and file copying.

## Features

- 🌲 **Easy worktree setup** - Create git worktrees with a single command
- 📦 **Automatic file copying** - Copies essential files (.env, .envrc, etc.) to new worktrees
- 🧹 **Smart cleanup** - Remove merged worktrees automatically or interactively
- ⏱️ **Activity-aware status** - Highlight stale worktrees with last activity timestamps
- 🎨 **Beautiful output** - Colored terminal output with progress indicators
- 💬 **Claude Comment Management** - Automatically minimize old Claude PR review comments

## Installation

### From crates.io (Recommended)
```bash
cargo install workbloom
```

### From GitHub
```bash
cargo install --git https://github.com/chaspy/workbloom.git
```

### From source
```bash
git clone https://github.com/chaspy/workbloom.git
cd workbloom
cargo install --path .
```

### Shell Integration (Recommended)

#### Alias for shorter commands

Add this alias to your `.bashrc` or `.zshrc` for shorter commands:

```bash
alias wb='workbloom'
```

With this alias and the built-in short aliases, you can use:
- `wb s feature/my-feature` instead of `workbloom setup feature/my-feature`
- `wb c` instead of `workbloom cleanup`

#### Auto-change directory after setup

To automatically change to the worktree directory after setup, add this function to your `.bashrc` or `.zshrc`:

```bash
workbloom-setup() {
    local output=$(workbloom setup "$@")
    echo "$output"
    
    # Extract the worktree path and change to it
    local worktree_path=$(echo "$output" | grep "📍 Worktree location:" | sed 's/.*: //')
    if [ -n "$worktree_path" ] && [ -d "$worktree_path" ]; then
        cd "$worktree_path"
        echo "📂 Changed to worktree directory: $(pwd)"
    fi
}

# Or with the alias:
wb-setup() {
    local output=$(wb s "$@")
    echo "$output"
    
    # Extract the worktree path and change to it
    local worktree_path=$(echo "$output" | grep "📍 Worktree location:" | sed 's/.*: //')
    if [ -n "$worktree_path" ] && [ -d "$worktree_path" ]; then
        cd "$worktree_path"
        echo "📂 Changed to worktree directory: $(pwd)"
    fi
}

# Then use: workbloom-setup feature/my-feature
# Or: wb-setup feature/my-feature
```

## Usage

### Setup a new worktree

```bash
# Setup and start a new shell in the worktree directory (default)
workbloom setup feature/my-new-feature
# Or using short alias: wb s feature/my-new-feature

# Setup without starting a shell
workbloom setup feature/my-new-feature --no-shell
# Or using short alias: wb s feature/my-new-feature --no-shell
```

This will:
1. Create a new worktree for the branch (creating the branch if it doesn't exist)
2. Copy required files from the main repository (.env, .envrc, etc.)
3. Setup direnv if available
4. Start a new shell in the worktree directory (unless --no-shell is used)

### Clean up worktrees

```bash
# Remove merged worktrees (default)
workbloom cleanup
# Or using short alias: wb c

# Force removal of merged worktrees (skip remote branch checks)
workbloom cleanup --merged --force
# Or using short alias: wb c --merged --force

# Remove worktrees matching a pattern
workbloom cleanup --pattern "feature/old-"
# Or using short alias: wb c --pattern "feature/old-"

# Interactive cleanup
workbloom cleanup --interactive
# Or using short alias: wb c --interactive

# Show merge status of all worktrees
workbloom cleanup --status
# Or using short alias: wb c --status
```

#### Cleanup Options

- **Default**: Removes only merged worktrees that exist on the remote repository
- **`--force`**: Skips remote branch checks and safety filters, removing all merged worktrees (use with caution)
  - Useful when remote branches have been deleted after merging
  - Still protects recently created worktrees (within 24 hours) via the filesystem age check
- **`--pattern`**: Removes worktrees matching the specified pattern
- **`--interactive`**: Prompts for confirmation before removing each worktree
- **`--status`**: Shows the merge status and last activity of all branches without removing anything, and optionally offers to delete stale ones

Sample status output:

```bash
📍 main (current branch)
✅ feature/login (merged, last activity 2h ago)
❌ issue-123 (not merged, last activity 21d ago ⚠️ stale)

🧭 The following worktrees have seen no activity for 14 days or more:
  - issue-123 (last activity 21d ago)

⏳ Branch 'issue-123' has been inactive for 21d ago
    Worktree path: /path/to/worktree-issue-123
    Remove this worktree? (y/N)
```

## Configuration

### Default Files

By default, Workbloom copies the following files to new worktrees:
- `.envrc`
- `.env`
- `.claude/settings.json`
- `.claude/settings.local.json`

### Custom File Copying

You can specify additional files and directories to copy by creating a `.workbloom` file in your repository root:

```bash
# .workbloom - List of files and directories to copy to git worktrees
# One file or directory per line
# Lines starting with # are comments
# Directories should end with /

# Example:
service-account.json
config/database.yml
.secret/credentials.json
certificates/
```

See `.workbloom.example` for a complete example.

## Development

```bash
# Run tests
cargo test

# Build debug version
cargo build

# Build release version
cargo build --release

# Run with debug output
RUST_LOG=debug cargo run -- setup test-branch
```

## Claude Comment Management

古いClaudeのPRレビューコメントが蓄積することを防ぐため、自動的に最小化する機能を提供します。

### 自動実行

GitHub Actionsが以下の場合に自動実行されます：
- PR作成・更新時
- PRコメント作成時

### 手動実行

```bash
cd scripts
npm install

# PR #123の古いClaudeコメントを最小化
node claude-comment-manager.js 123

# 詳細な使用方法
node claude-comment-manager.js --help
```

詳細については [scripts/README.md](scripts/README.md) を参照してください。

## License

MIT
