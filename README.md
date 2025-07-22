# Workbloom

[![Build](https://github.com/chaspy/workbloom/actions/workflows/build.yml/badge.svg)](https://github.com/chaspy/workbloom/actions/workflows/build.yml)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=chaspy_workbloom&metric=alert_status)](https://sonarcloud.io/dashboard?id=chaspy_workbloom)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=chaspy_workbloom&metric=coverage)](https://sonarcloud.io/dashboard?id=chaspy_workbloom)
[![Crates.io](https://img.shields.io/crates/v/workbloom.svg)](https://crates.io/crates/workbloom)

A Git worktree management tool written in Rust that automates worktree setup, file copying, and port allocation.

## Features

- 🌲 **Easy worktree setup** - Create git worktrees with a single command
- 📦 **Automatic file copying** - Copies essential files (.env, .envrc, etc.) to new worktrees
- 🌐 **Port allocation** - Automatically assigns unique ports based on branch names
- 🧹 **Smart cleanup** - Remove merged worktrees automatically or interactively
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

# Then use: workbloom-setup feature/my-feature
```

## Usage

### Setup a new worktree

```bash
# Setup and start a new shell in the worktree directory (default)
workbloom setup feature/my-new-feature

# Setup without starting a shell
workbloom setup feature/my-new-feature --no-shell
```

This will:
1. Create a new worktree for the branch (creating the branch if it doesn't exist)
2. Copy required files from the main repository (.env, .envrc, etc.)
3. Setup direnv if available
4. Write port allocations to .env file
5. Display allocated ports for the worktree
6. Start a new shell in the worktree directory (unless --no-shell is used)

### Clean up worktrees

```bash
# Remove merged worktrees (default)
workbloom cleanup

# Remove worktrees matching a pattern
workbloom cleanup --pattern "feature/old-"

# Interactive cleanup
workbloom cleanup --interactive

# Show merge status of all worktrees
workbloom cleanup --status
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

## Port Allocation

Workbloom automatically allocates unique ports for each worktree based on the branch name:
- Frontend: 5173 + hash
- Backend: 8080 + hash
- Database: 5432 + hash

The same branch name will always get the same ports. These port allocations are automatically written to the `.env` file:
```
FRONTEND_PORT=6174
BACKEND_PORT=9081
DATABASE_PORT=6433
```

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