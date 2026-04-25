# git-ops

A crate providing ergonomics around common git2 operations.

## Features

| Feature | Description |
|---|---|
| **Git Client** | Simple interface for git operations |
| **Clone Operations** | Clone repositories via SSH or HTTPS |
| **Branch Checkout** | Checkout branches in repositories |
| **Pull Operations** | Pull changes from remote repositories |
| **Commit Operations** | Stage all changes and create commits, with optional GPG/SSH signing |
| **SSH Authentication** | Built-in SSH credential handling |
| **HTTPS Authentication** | Credential helper and environment variable support |
| **Error Handling** | Comprehensive error types for git operations |

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

// Clone a repository (SSH or HTTPS detected automatically)
client.clone("git@github.com:user/repo.git", Path::new("/path/to/repo"))?;

// Pull changes
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
- `clone(url, destination)` - Clones a repository to the given path (SSH or HTTPS)
- `pull(repo_path)` - Pulls updates for an existing repository
- `checkout_branch(repo_path, branch_name)` - Checkouts a branch in the repository
- `commit(repo_path, message)` - Stages all changes and creates a commit; signs if `commit.gpgsign = true` in git config
- `extract_repo_name(url)` - Extracts the repository name from a Git URL
- `convert_ssh_to_https(url)` - Converts an SSH URL to HTTPS
- `is_valid_git_url(url)` - Validates that a string is a recognized Git URL
- `has_https_credentials()` - Checks if HTTPS credentials are available

HTTPS authentication tries git credential helpers first, then falls back to the `GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_ACCESS_TOKEN` environment variables.

### Other Types

- **`SshConfig`**: SSH authentication configuration
- **`GitError`**: Error type for git operations

## Requirements

- Rust 2021 edition or later
- libgit2 (provided via git2-rs)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

