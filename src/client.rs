use std::path::Path;

use git2::Repository;

use crate::auth::SshConfig;
use crate::checkout::GitCheckout;
use crate::clone::GitCloner;
use crate::commit::GitCommit;
use crate::error::GitError;
use crate::pull::GitPuller;
use crate::push::{GitPusher, PushOptions};

/// Git operations client that handles repository cloning, pulling, and checkout with SSH authentication
pub struct GitClient {
    puller: GitPuller,
    cloner: GitCloner,
    pusher: GitPusher,
}

impl GitClient {
    /// Create a new GitClient with default SSH configuration
    pub fn new() -> Result<Self, GitError> {
        let ssh_config = SshConfig::from_environment()?;
        let puller = GitPuller::new(ssh_config.clone());
        let cloner = GitCloner::new(ssh_config.clone());
        let pusher = GitPusher::new(ssh_config);

        Ok(Self {
            puller,
            cloner,
            pusher,
        })
    }

    /// Create a new GitClient with custom SSH configuration
    pub fn with_ssh_config(ssh_config: SshConfig) -> Self {
        let puller = GitPuller::new(ssh_config.clone());
        let cloner = GitCloner::new(ssh_config.clone());
        let pusher = GitPusher::new(ssh_config);
        Self {
            puller,
            cloner,
            pusher,
        }
    }

    /// Clone a repository to the given destination path
    ///
    /// # Arguments
    /// * `url` - The Git repository URL to clone
    /// * `destination` - The path to clone the repository into
    pub fn clone(&self, url: &str, destination: &Path) -> Result<Repository, GitError> {
        self.cloner.clone(url, destination)
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

    /// Push the current branch to its configured remote (defaults to `origin`).
    pub fn push(&self, repo_path: &Path) -> Result<(), GitError> {
        self.pusher.push(repo_path)
    }

    /// Push with full control over remote, branch, force, and upstream options.
    pub fn push_with_options(
        &self,
        repo_path: &Path,
        options: &PushOptions,
    ) -> Result<(), GitError> {
        self.pusher.push_with_options(repo_path, options)
    }

    /// Commit staged and unstaged changes in the repository.
    ///
    /// Signing is enabled only when `commit.gpgsign = true` is set in git config.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository
    /// * `message` - The commit message
    pub fn commit(&self, repo_path: &Path, message: &str) -> Result<(), GitError> {
        GitCommit::commit(repo_path, message)
    }

    /// Extract a repository name from a Git URL
    pub fn extract_repo_name(url: &str) -> Result<String, GitError> {
        GitCloner::extract_repo_name(url)
    }

    /// Convert an SSH URL to HTTPS, returns None if the URL is not SSH
    pub fn convert_ssh_to_https(url: &str) -> Option<String> {
        GitCloner::convert_ssh_to_https(url)
    }

    /// Validate that a URL looks like a valid Git URL
    pub fn is_valid_git_url(url: &str) -> bool {
        GitCloner::is_valid_git_url(url)
    }

    /// Check if HTTPS credentials are available via credential helper or environment variables
    pub fn has_https_credentials() -> bool {
        GitCloner::has_https_credentials()
    }
}

impl Default for GitClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default GitClient")
    }
}
