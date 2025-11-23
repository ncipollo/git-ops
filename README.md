# git-ops

A Rust library for performing common git operations with support for SSH authentication.

## Features

- **Git Client**: Simple interface for git operations
- **Branch Checkout**: Checkout branches in repositories
- **Pull Operations**: Pull changes from remote repositories
- **SSH Authentication**: Built-in SSH credential handling
- **Error Handling**: Comprehensive error types for git operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
git-ops = "0.1.0"
```

## Usage

### Basic Usage

```rust
use git_ops::GitClient;
use std::path::Path;

// Create a git client (uses SSH config from environment)
let client = GitClient::new()?;

// Pull changes from an existing repository
client.pull(Path::new("/path/to/repo"))?;

// Checkout a branch
client.checkout_branch(Path::new("/path/to/repo"), "main")?;
```

### Custom SSH Configuration

```rust
use git_ops::{GitClient, SshConfig};
use std::path::Path;

// Create SSH config with custom key paths
let ssh_config = SshConfig::from_environment()?;

// Create client with custom SSH config
let client = GitClient::with_ssh_config(ssh_config);

// Use the client
client.pull(Path::new("/path/to/repo"))?;
```

## API Documentation

### GitClient

The main client for git operations:

- `GitClient::new()` - Creates a client with SSH config from environment variables
- `GitClient::with_ssh_config(ssh_config)` - Creates a client with custom SSH configuration
- `pull(repo_path)` - Pulls updates for an existing repository
- `checkout_branch(repo_path, branch_name)` - Checkouts a branch in the repository

### Other Types

- **`SshConfig`**: SSH authentication configuration
- **`GitError`**: Error type for git operations

## Requirements

- Rust 2021 edition or later
- libgit2 (provided via git2-rs)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

