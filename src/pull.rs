use git2::Repository;
use std::path::Path;

use crate::auth::{CredentialCallback, SshConfig};
use crate::error::GitError;

/// Pull operations for Git repositories
pub struct GitPuller {
    ssh_config: SshConfig,
}

impl GitPuller {
    /// Create a new GitPuller with the provided SSH configuration
    pub fn new(ssh_config: SshConfig) -> Self {
        Self { ssh_config }
    }

    /// Pull updates for an existing repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository to update
    pub fn pull(&self, repo_path: &Path) -> Result<(), GitError> {
        let repo = Repository::open(repo_path).map_err(|e| GitError::OpenFailed {
            path: repo_path.to_path_buf(),
            source: e,
        })?;

        // Get the current branch
        let head = repo.head().map_err(|e| GitError::PullFailed {
            path: repo_path.to_path_buf(),
            source: e,
        })?;

        let branch_name = head
            .shorthand()
            .ok_or_else(|| GitError::InvalidBranch(repo_path.to_path_buf()))?;

        // Find the remote (assume origin)
        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| GitError::PullFailed {
                path: repo_path.to_path_buf(),
                source: e,
            })?;

        // Set up fetch options with appropriate authentication based on remote URL
        let mut fetch_options = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Get remote URL to determine authentication strategy
        let remote_url = remote.url().unwrap_or("");

        if Self::is_https_url(remote_url) {
            // Try HTTPS authentication (with PAT fallback)
            if let Ok(credentials_callback) = Self::https_credentials_callback() {
                callbacks.credentials(credentials_callback);
            }
        } else {
            // Use SSH authentication
            let credentials_callback = self.ssh_config.credentials_callback()?;
            callbacks.credentials(credentials_callback);
        }

        fetch_options.remote_callbacks(callbacks);

        // Fetch all branches from remote
        // Using empty slice fetches all configured refspecs (all branches)
        remote
            .fetch(&[] as &[&str], Some(&mut fetch_options), None)
            .map_err(|e| GitError::PullFailed {
                path: repo_path.to_path_buf(),
                source: e,
            })?;

        // Get the fetch head and merge
        repo.fetchhead_foreach(|_ref_name, _remote_url, _oid, _is_merge| {
            // Fetch head processing - we'll use this for more advanced merging later
            true
        })
        .map_err(|e| GitError::PullFailed {
            path: repo_path.to_path_buf(),
            source: e,
        })?;

        // Get the remote branch reference that was just fetched
        let remote_branch_name = format!("refs/remotes/origin/{branch_name}");
        let remote_ref =
            repo.find_reference(&remote_branch_name)
                .map_err(|e| GitError::PullFailed {
                    path: repo_path.to_path_buf(),
                    source: e,
                })?;

        // Create annotated commit from the remote branch (not local HEAD)
        let annotated_commit = repo
            .reference_to_annotated_commit(&remote_ref)
            .map_err(|e| GitError::PullFailed {
                path: repo_path.to_path_buf(),
                source: e,
            })?;

        // Perform the merge (fast-forward only for now)
        let analysis =
            repo.merge_analysis(&[&annotated_commit])
                .map_err(|e| GitError::PullFailed {
                    path: repo_path.to_path_buf(),
                    source: e,
                })?;

        if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{branch_name}");
            let mut reference =
                repo.find_reference(&refname)
                    .map_err(|e| GitError::PullFailed {
                        path: repo_path.to_path_buf(),
                        source: e,
                    })?;

            reference
                .set_target(annotated_commit.id(), "Fast-forward")
                .map_err(|e| GitError::PullFailed {
                    path: repo_path.to_path_buf(),
                    source: e,
                })?;

            repo.set_head(&refname).map_err(|e| GitError::PullFailed {
                path: repo_path.to_path_buf(),
                source: e,
            })?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| GitError::PullFailed {
                    path: repo_path.to_path_buf(),
                    source: e,
                })?;
        } else if analysis.0.is_up_to_date() {
            // Already up to date, nothing to do
        } else {
            return Err(GitError::MergeRequired(repo_path.to_path_buf()));
        }

        Ok(())
    }

    /// Get git config and check for credential helper configuration
    fn get_git_config_with_credential_helpers() -> Result<git2::Config, git2::Error> {
        let config = git2::Config::open_default().or_else(|_| git2::Config::new())?;

        // Check if credential helpers are configured
        let has_credential_helper = config.get_string("credential.helper").is_ok()
            || config.entries(Some("credential\\..*\\.helper")).is_ok();

        if !has_credential_helper {
            eprintln!("Warning: No git credential helpers configured. Consider setting up a credential helper for better authentication:");
            eprintln!("  git config --global credential.helper store");
            eprintln!("  git config --global credential.helper cache");
            eprintln!("  git config --global credential.helper osxkeychain  # macOS");
            eprintln!("  git config --global credential.helper manager-core  # Cross-platform");
        }

        Ok(config)
    }

    /// Create credentials callback for HTTPS authentication using Git credential manager
    fn https_credentials_callback() -> Result<CredentialCallback, GitError> {
        Ok(Box::new(
            |url: &str, username_from_url: Option<&str>, allowed_types: git2::CredentialType| {
                // Try git credential helper first
                if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    if let Ok(config) = Self::get_git_config_with_credential_helpers() {
                        if let Ok(cred) =
                            git2::Cred::credential_helper(&config, url, username_from_url)
                        {
                            return Ok(cred);
                        }
                    }
                }

                // Fallback to environment variables for backward compatibility
                if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                        // For GitHub PAT, username can be anything (token is what matters)
                        let username = username_from_url.unwrap_or("git");
                        if let Ok(cred) = git2::Cred::userpass_plaintext(username, &token) {
                            return Ok(cred);
                        }
                    }

                    // Try alternative environment variable names
                    if let Ok(token) = std::env::var("GH_TOKEN") {
                        let username = username_from_url.unwrap_or("git");
                        if let Ok(cred) = git2::Cred::userpass_plaintext(username, &token) {
                            return Ok(cred);
                        }
                    }

                    if let Ok(token) = std::env::var("GITHUB_ACCESS_TOKEN") {
                        let username = username_from_url.unwrap_or("git");
                        if let Ok(cred) = git2::Cred::userpass_plaintext(username, &token) {
                            return Ok(cred);
                        }
                    }
                }

                // Try default credentials
                if allowed_types.contains(git2::CredentialType::DEFAULT) {
                    if let Ok(cred) = git2::Cred::default() {
                        return Ok(cred);
                    }
                }

                // If we get here, authentication failed
                Err(git2::Error::from_str("No HTTPS credentials found. Configure git credential helper or set GITHUB_TOKEN environment variable for private repositories."))
            },
        ))
    }

    /// Check if URL is HTTPS
    fn is_https_url(url: &str) -> bool {
        url.starts_with("https://")
    }
}

