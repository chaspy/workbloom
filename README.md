# Workbloom

A Git worktree management tool written in Rust that automates worktree setup, file copying, and port allocation.

## Features

- 🌲 **Easy worktree setup** - Create git worktrees with a single command
- 📦 **Automatic file copying** - Copies essential files (.env, .envrc, etc.) to new worktrees
- 🌐 **Port allocation** - Automatically assigns unique ports based on branch names
- 🧹 **Smart cleanup** - Remove merged worktrees automatically or interactively
- 🎨 **Beautiful output** - Colored terminal output with progress indicators

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/workbloom.git
cd workbloom

# Build and install
cargo install --path .
```

## Usage

### Setup a new worktree

```bash
workbloom setup feature/my-new-feature
```

This will:
1. Create a new worktree for the branch (creating the branch if it doesn't exist)
2. Copy required files from the main repository (.env, .envrc, etc.)
3. Setup direnv if available
4. Display allocated ports for the worktree

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

By default, Workbloom copies the following files to new worktrees:
- `.envrc`
- `.env`
- `.claude/settings.json`
- `.claude/settings.local.json`

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

## License

MIT