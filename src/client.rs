use std::path::Path;

use crate::auth::SshConfig;
use crate::checkout::GitCheckout;
use crate::error::GitError;
use crate::pull::GitPuller;

/// Git operations client that handles repository pulling and checkout with SSH authentication
pub struct GitClient {
    puller: GitPuller,
}

impl GitClient {
    /// Create a new GitClient with default SSH configuration
    pub fn new() -> Result<Self, GitError> {
        let ssh_config = SshConfig::from_environment()?;
        let puller = GitPuller::new(ssh_config);

        Ok(Self { puller })
    }

    /// Create a new GitClient with custom SSH configuration
    pub fn with_ssh_config(ssh_config: SshConfig) -> Self {
        let puller = GitPuller::new(ssh_config);
        Self { puller }
    }

    /// Pull updates for an existing repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository to update
    pub fn pull(&self, repo_path: &Path) -> Result<(), GitError> {
        self.puller.pull(repo_path)
    }

    /// Checkout a branch in the repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository
    /// * `branch_name` - Name of the branch to checkout
    ///
    /// # Errors
    /// Returns an error if the branch doesn't exist or checkout fails
    pub fn checkout_branch(&self, repo_path: &Path, branch_name: &str) -> Result<(), GitError> {
        GitCheckout::checkout_branch(repo_path, branch_name)
    }
}

impl Default for GitClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default GitClient")
    }
}

