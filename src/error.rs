use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during Git operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("SSH error: {0}")]
    Ssh(#[from] SshError),

    #[error("Failed to open repository at {path}: {source}")]
    OpenFailed {
        path: PathBuf,
        #[source]
        source: git2::Error,
    },

    #[error("Failed to pull repository at {path}: {source}")]
    PullFailed {
        path: PathBuf,
        #[source]
        source: git2::Error,
    },

    #[error("Invalid branch for repository at {0}")]
    InvalidBranch(PathBuf),

    #[error("Manual merge required for repository at {0}")]
    MergeRequired(PathBuf),

    #[error("Failed to checkout branch {branch} at {path}: {source}")]
    CheckoutFailed {
        branch: String,
        path: PathBuf,
        #[source]
        source: git2::Error,
    },

    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),
}

/// Errors that can occur during SSH operations
#[derive(Error, Debug)]
pub enum SshError {
    #[error("Home directory not found")]
    HomeDirectoryNotFound,

    #[error("SSH directory not found: {0}")]
    SshDirectoryNotFound(PathBuf),

    #[error("No SSH credentials available (no keys found and SSH agent disabled)")]
    NoCredentialsAvailable,

    #[error("SSH key not found: {0}")]
    KeyNotFound(PathBuf),

    #[error("SSH key permissions invalid: {0}")]
    InvalidKeyPermissions(PathBuf),

    #[error("SSH authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("SSH agent connection failed: {0}")]
    AgentConnectionFailed(String),

    #[error("SSH key passphrase required: {0}")]
    PassphraseRequired(PathBuf),

    #[error("SSH configuration invalid: {0}")]
    InvalidConfiguration(String),
}

impl GitError {
    /// Get a user-friendly error message with suggestions
    pub fn user_message(&self) -> String {
        match self {
            GitError::OpenFailed { path, .. } => {
                format!(
                    "Failed to open repository at {}. Make sure it's a valid Git repository.",
                    path.display()
                )
            }
            GitError::PullFailed { path, .. } => {
                format!(
                    "Failed to pull updates for repository at {}. Check your SSH keys and network connection.",
                    path.display()
                )
            }
            GitError::MergeRequired(path) => {
                format!(
                    "Manual merge required for repository at {}. Resolve conflicts manually.",
                    path.display()
                )
            }
            GitError::CheckoutFailed { branch, path, .. } => {
                format!(
                    "Failed to checkout branch '{}' at {}. Check if the branch exists.",
                    branch,
                    path.display()
                )
            }
            GitError::Ssh(ssh_error) => ssh_error.user_message(),
            _ => self.to_string(),
        }
    }
}

impl SshError {
    /// Get a user-friendly error message with suggestions
    pub fn user_message(&self) -> String {
        match self {
            SshError::HomeDirectoryNotFound => {
                "Home directory not found. Make sure your environment is properly configured.".to_string()
            }
            SshError::SshDirectoryNotFound(path) => {
                format!(
                    "SSH directory not found: {}. Create it with: mkdir -p {}",
                    path.display(),
                    path.display()
                )
            }
            SshError::NoCredentialsAvailable => {
                "No SSH credentials available. Generate SSH keys with: ssh-keygen -t ed25519 -C \"your_email@example.com\"".to_string()
            }
            SshError::KeyNotFound(path) => {
                format!(
                    "SSH key not found: {}. Generate it with: ssh-keygen -t ed25519 -f {}",
                    path.display(),
                    path.display()
                )
            }
            SshError::InvalidKeyPermissions(path) => {
                format!(
                    "SSH key has invalid permissions: {}. Fix with: chmod 600 {}",
                    path.display(),
                    path.display()
                )
            }
            SshError::AuthenticationFailed(details) => {
                format!(
                    "SSH authentication failed: {details}. Check your SSH keys and add them to the Git service."
                )
            }
            SshError::AgentConnectionFailed(details) => {
                format!(
                    "SSH agent connection failed: {details}. Start SSH agent with: eval \"$(ssh-agent -s)\""
                )
            }
            SshError::PassphraseRequired(path) => {
                format!(
                    "SSH key passphrase required: {}. Add to SSH agent with: ssh-add {}",
                    path.display(),
                    path.display()
                )
            }
            SshError::InvalidConfiguration(details) => {
                format!(
                    "SSH configuration invalid: {details}. Check your SSH configuration."
                )
            }
        }
    }
}

