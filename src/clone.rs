use git2::Repository;
use std::path::Path;

use crate::auth::SshConfig;
use crate::error::GitError;
use crate::https;

/// Clone operations for Git repositories
pub struct GitCloner {
    ssh_config: SshConfig,
}

impl GitCloner {
    /// Create a new GitCloner with the provided SSH configuration
    pub fn new(ssh_config: SshConfig) -> Self {
        Self { ssh_config }
    }

    /// Clone a repository to the given destination path
    ///
    /// # Arguments
    /// * `url` - The Git repository URL to clone
    /// * `destination` - The path to clone the repository into
    ///
    /// # Returns
    /// The cloned Repository
    pub fn clone(&self, url: &str, destination: &Path) -> Result<Repository, GitError> {
        if destination.exists() {
            return Err(GitError::RepositoryExists(destination.to_path_buf()));
        }

        if https::is_https_url(url) {
            self.clone_https(url, destination)
        } else {
            self.clone_ssh(url, destination)
        }
    }

    /// Clone repository using SSH authentication
    fn clone_ssh(&self, url: &str, repo_path: &Path) -> Result<Repository, GitError> {
        let mut fetch_options = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        let credentials_callback = self.ssh_config.credentials_callback()?;
        callbacks.credentials(credentials_callback);

        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        builder
            .clone(url, repo_path)
            .map_err(|e| GitError::CloneFailed {
                url: url.to_string(),
                path: repo_path.to_path_buf(),
                source: e,
            })
    }

    /// Clone repository using HTTPS authentication
    fn clone_https(&self, url: &str, repo_path: &Path) -> Result<Repository, GitError> {
        let mut fetch_options = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        let credentials_callback = https::https_credentials_callback()?;
        callbacks.credentials(credentials_callback);

        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        builder
            .clone(url, repo_path)
            .map_err(|e| GitError::CloneFailed {
                url: url.to_string(),
                path: repo_path.to_path_buf(),
                source: e,
            })
    }

    /// Check if HTTPS credentials are available
    pub fn has_https_credentials() -> bool {
        https::has_https_credentials()
    }

    /// Validate that a URL looks like a valid Git URL
    pub fn is_valid_git_url(url: &str) -> bool {
        let has_valid_protocol = url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("git@")
            || url.starts_with("ssh://")
            || url.starts_with("git://")
            || (url.starts_with("file://") && url.len() > 7);

        if !has_valid_protocol {
            return false;
        }

        if url.starts_with("http://") || url.starts_with("https://") {
            let parts: Vec<&str> = url.split('/').collect();
            // Should have at least: ["https:", "", "host", "user", "repo"]
            if parts.len() < 5 {
                return false;
            }
            let last_part = parts.last().unwrap_or(&"").trim_end_matches(".git");
            if last_part.is_empty() {
                return false;
            }
        }

        // For SSH URLs like git@github.com:user/repo.git
        if url.starts_with("git@") && (!url.contains(':') || !url.contains('/')) {
            return false;
        }

        true
    }

    /// Convert an SSH URL to HTTPS, returns None if the URL is not SSH
    pub fn convert_ssh_to_https(url: &str) -> Option<String> {
        // Handle git@host:user/repo.git format
        if url.starts_with("git@") && url.contains(':') && !url.starts_with("ssh://") {
            if let Some((host_part, path_part)) = url.split_once(':') {
                let host = host_part.strip_prefix("git@")?;
                return Some(format!("https://{host}/{path_part}"));
            }
        }

        // Handle ssh://git@host/user/repo.git format
        if url.starts_with("ssh://") {
            if let Some(ssh_url) = url.strip_prefix("ssh://") {
                if ssh_url.starts_with("git@") {
                    if let Some(host_and_path) = ssh_url.strip_prefix("git@") {
                        if let Some((host, path)) = host_and_path.split_once('/') {
                            return Some(format!("https://{host}/{path}"));
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract and sanitize a repository name from a Git URL
    pub fn extract_repo_name(url: &str) -> Result<String, GitError> {
        let url = url.trim_end_matches('/');

        if !Self::is_valid_git_url(url) {
            return Err(GitError::InvalidUrl(url.to_string()));
        }

        let name = url
            .split('/')
            .next_back()
            .ok_or_else(|| GitError::InvalidUrl(url.to_string()))?
            .trim_end_matches(".git");

        if name.is_empty() {
            return Err(GitError::InvalidUrl(url.to_string()));
        }

        let sanitized = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect::<String>();

        if sanitized.is_empty() {
            return Err(GitError::InvalidUrl(url.to_string()));
        }

        Ok(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name() {
        assert_eq!(
            GitCloner::extract_repo_name("https://github.com/user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            GitCloner::extract_repo_name("git@github.com:user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            GitCloner::extract_repo_name("https://github.com/user/repo").unwrap(),
            "repo"
        );
        assert_eq!(
            GitCloner::extract_repo_name("ssh://git@example.com/path/to/repo.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_sanitization() {
        assert_eq!(
            GitCloner::extract_repo_name("https://github.com/user/repo-name_123.git").unwrap(),
            "repo-name_123"
        );
        assert_eq!(
            GitCloner::extract_repo_name("https://github.com/user/repo$@#!.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_invalid() {
        assert!(GitCloner::extract_repo_name("").is_err());
        assert!(GitCloner::extract_repo_name("invalid").is_err());
        assert!(GitCloner::extract_repo_name("https://github.com/user/").is_err());
    }

    #[test]
    fn test_is_valid_git_url() {
        assert!(GitCloner::is_valid_git_url(
            "https://github.com/user/repo.git"
        ));
        assert!(GitCloner::is_valid_git_url(
            "http://github.com/user/repo.git"
        ));
        assert!(GitCloner::is_valid_git_url("git@github.com:user/repo.git"));
        assert!(GitCloner::is_valid_git_url(
            "ssh://git@github.com/user/repo.git"
        ));
        assert!(GitCloner::is_valid_git_url(
            "git://github.com/user/repo.git"
        ));
        assert!(GitCloner::is_valid_git_url("file:///path/to/repo.git"));

        assert!(!GitCloner::is_valid_git_url(""));
        assert!(!GitCloner::is_valid_git_url("invalid"));
        assert!(!GitCloner::is_valid_git_url("https://github.com/user/"));
        assert!(!GitCloner::is_valid_git_url("git@github.com"));
    }

    #[test]
    fn test_is_https_url() {
        assert!(https::is_https_url("https://github.com/user/repo.git"));
        assert!(https::is_https_url("https://gitlab.com/user/repo.git"));

        assert!(!https::is_https_url("git@github.com:user/repo.git"));
        assert!(!https::is_https_url("ssh://git@github.com/user/repo.git"));
        assert!(!https::is_https_url("http://github.com/user/repo.git"));
        assert!(!https::is_https_url(""));
    }

    #[test]
    fn test_convert_ssh_to_https() {
        assert_eq!(
            GitCloner::convert_ssh_to_https("git@github.com:user/repo.git"),
            Some("https://github.com/user/repo.git".to_string())
        );
        assert_eq!(
            GitCloner::convert_ssh_to_https("git@gitlab.com:user/repo.git"),
            Some("https://gitlab.com/user/repo.git".to_string())
        );

        assert_eq!(
            GitCloner::convert_ssh_to_https("ssh://git@github.com/user/repo.git"),
            Some("https://github.com/user/repo.git".to_string())
        );
        assert_eq!(
            GitCloner::convert_ssh_to_https("ssh://git@gitlab.com/user/repo.git"),
            Some("https://gitlab.com/user/repo.git".to_string())
        );

        assert_eq!(
            GitCloner::convert_ssh_to_https("https://github.com/user/repo.git"),
            None
        );
        assert_eq!(
            GitCloner::convert_ssh_to_https("http://github.com/user/repo.git"),
            None
        );
        assert_eq!(GitCloner::convert_ssh_to_https(""), None);
        assert_eq!(GitCloner::convert_ssh_to_https("invalid"), None);
        assert_eq!(GitCloner::convert_ssh_to_https("git@github.com"), None);
        assert_eq!(GitCloner::convert_ssh_to_https("ssh://github.com"), None);
    }
}
