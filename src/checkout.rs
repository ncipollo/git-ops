use git2::Repository;
use std::path::Path;

use crate::error::GitError;

/// Checkout operations for Git repositories
pub struct GitCheckout;

impl GitCheckout {
    /// Checkout a branch in the repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to the repository
    /// * `branch_name` - Name of the branch to checkout
    ///
    /// # Errors
    /// Returns an error if the branch doesn't exist or checkout fails
    pub fn checkout_branch(repo_path: &Path, branch_name: &str) -> Result<(), GitError> {
        let repo = Repository::open(repo_path).map_err(|e| GitError::OpenFailed {
            path: repo_path.to_path_buf(),
            source: e,
        })?;

        // Try to find the branch as a local branch first
        let branch_ref = format!("refs/heads/{}", branch_name);
        let remote_branch_ref = format!("refs/remotes/origin/{}", branch_name);

        // Check if local branch exists
        let reference = if repo.find_reference(&branch_ref).is_ok() {
            repo.find_reference(&branch_ref)
                .map_err(|e| GitError::CheckoutFailed {
                    branch: branch_name.to_string(),
                    path: repo_path.to_path_buf(),
                    source: e,
                })?
        } else if let Ok(remote_ref) = repo.find_reference(&remote_branch_ref) {
            // If remote branch exists, create local branch from it
            let remote_commit =
                remote_ref
                    .peel_to_commit()
                    .map_err(|e| GitError::CheckoutFailed {
                        branch: branch_name.to_string(),
                        path: repo_path.to_path_buf(),
                        source: e,
                    })?;

            repo.branch(branch_name, &remote_commit, false)
                .map_err(|e| GitError::CheckoutFailed {
                    branch: branch_name.to_string(),
                    path: repo_path.to_path_buf(),
                    source: e,
                })?;

            repo.find_reference(&branch_ref)
                .map_err(|e| GitError::CheckoutFailed {
                    branch: branch_name.to_string(),
                    path: repo_path.to_path_buf(),
                    source: e,
                })?
        } else {
            return Err(GitError::CheckoutFailed {
                branch: branch_name.to_string(),
                path: repo_path.to_path_buf(),
                source: git2::Error::from_str(&format!(
                    "Branch '{}' not found locally or remotely",
                    branch_name
                )),
            });
        };

        // Set HEAD to the branch
        let ref_name = reference.name().ok_or_else(|| {
            GitError::CheckoutFailed {
                branch: branch_name.to_string(),
                path: repo_path.to_path_buf(),
                source: git2::Error::from_str("Reference has no name"),
            }
        })?;
        repo.set_head(ref_name)
            .map_err(|e| GitError::CheckoutFailed {
                branch: branch_name.to_string(),
                path: repo_path.to_path_buf(),
                source: e,
            })?;

        // Checkout the branch
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| GitError::CheckoutFailed {
                branch: branch_name.to_string(),
                path: repo_path.to_path_buf(),
                source: e,
            })?;

        Ok(())
    }
}

