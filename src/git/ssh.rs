use std::path::PathBuf;

use git2::{Cred, CredentialType};

use crate::git::{GitError, SshError};

/// Type alias for the SSH credentials callback function
type CredentialsCallback =
    dyn FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, git2::Error>;

/// SSH configuration for Git operations
#[derive(Debug, Clone)]
pub struct SshConfig {
    /// Paths to private SSH keys to try
    private_key_paths: Vec<PathBuf>,
    /// Path to the known_hosts file
    known_hosts_path: PathBuf,
    /// Whether to use SSH agent if available
    ssh_agent: bool,
}

impl SshConfig {
    /// Create SSH configuration from environment
    pub fn from_environment() -> Result<Self, SshError> {
        let home_dir = dirs::home_dir().ok_or(SshError::HomeDirectoryNotFound)?;

        let ssh_dir = home_dir.join(".ssh");

        // Standard SSH key locations to try
        let private_key_paths = vec![
            ssh_dir.join("id_ed25519"),
            ssh_dir.join("id_rsa"),
            ssh_dir.join("id_ecdsa"),
            ssh_dir.join("id_dsa"),
        ];

        let known_hosts_path = ssh_dir.join("known_hosts");

        Ok(Self {
            private_key_paths,
            known_hosts_path,
            ssh_agent: true,
        })
    }

    /// Create SSH configuration with custom settings
    pub fn new(
        private_key_paths: Vec<PathBuf>,
        known_hosts_path: PathBuf,
        ssh_agent: bool,
    ) -> Self {
        Self {
            private_key_paths,
            known_hosts_path,
            ssh_agent,
        }
    }

    /// Create a credentials callback for Git operations
    pub fn credentials_callback(&self) -> Result<Box<CredentialsCallback>, GitError> {
        let ssh_config = self.clone();

        Ok(Box::new(
            move |_url: &str, username_from_url: Option<&str>, allowed_types: CredentialType| {
                // Try SSH agent first if enabled and allowed
                if ssh_config.ssh_agent && allowed_types.contains(CredentialType::SSH_KEY) {
                    if let Ok(cred) = Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")) {
                        return Ok(cred);
                    }
                }

                // Try SSH keys if allowed
                if allowed_types.contains(CredentialType::SSH_KEY) {
                    let username = username_from_url.unwrap_or("git");

                    for private_key_path in &ssh_config.private_key_paths {
                        if private_key_path.exists() {
                            let public_key_path = private_key_path.with_extension("pub");

                            // Try with public key if it exists
                            if public_key_path.exists() {
                                if let Ok(cred) = Cred::ssh_key(
                                    username,
                                    Some(&public_key_path),
                                    private_key_path,
                                    None, // No passphrase support for now
                                ) {
                                    return Ok(cred);
                                }
                            } else {
                                // Try without public key
                                if let Ok(cred) = Cred::ssh_key(
                                    username,
                                    None,
                                    private_key_path,
                                    None, // No passphrase support for now
                                ) {
                                    return Ok(cred);
                                }
                            }
                        }
                    }
                }

                // Try default credentials if allowed
                if allowed_types.contains(CredentialType::DEFAULT) {
                    if let Ok(cred) = Cred::default() {
                        return Ok(cred);
                    }
                }

                // If we get here, authentication failed
                Err(git2::Error::from_str("No valid credentials found"))
            },
        ))
    }

    /// Get the private key paths
    pub fn private_key_paths(&self) -> &[PathBuf] {
        &self.private_key_paths
    }

    /// Get the known hosts path
    pub fn known_hosts_path(&self) -> &PathBuf {
        &self.known_hosts_path
    }

    /// Check if SSH agent is enabled
    pub fn ssh_agent_enabled(&self) -> bool {
        self.ssh_agent
    }

    /// Add a private key path
    pub fn add_private_key_path(&mut self, path: PathBuf) {
        self.private_key_paths.push(path);
    }

    /// Set whether to use SSH agent
    pub fn set_ssh_agent(&mut self, enabled: bool) {
        self.ssh_agent = enabled;
    }

    /// Validate the SSH configuration
    pub fn validate(&self) -> Result<(), SshError> {
        // Check if at least one private key exists or SSH agent is enabled
        let has_keys = self.private_key_paths.iter().any(|path| path.exists());

        if !has_keys && !self.ssh_agent {
            return Err(SshError::NoCredentialsAvailable);
        }

        // Check if known_hosts directory exists (file doesn't need to exist)
        if let Some(parent) = self.known_hosts_path.parent() {
            if !parent.exists() {
                return Err(SshError::SshDirectoryNotFound(parent.to_path_buf()));
            }
        }

        Ok(())
    }
}

impl Default for SshConfig {
    fn default() -> Self {
        Self::from_environment().expect("Failed to create default SSH configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_ssh_config_creation() {
        let config = SshConfig::new(
            vec![PathBuf::from("/test/id_rsa")],
            PathBuf::from("/test/known_hosts"),
            true,
        );

        assert_eq!(config.private_key_paths(), &[PathBuf::from("/test/id_rsa")]);
        assert_eq!(
            config.known_hosts_path(),
            &PathBuf::from("/test/known_hosts")
        );
        assert!(config.ssh_agent_enabled());
    }

    #[test]
    fn test_ssh_config_modification() {
        let mut config = SshConfig::new(vec![], PathBuf::from("/test/known_hosts"), false);

        config.add_private_key_path(PathBuf::from("/test/id_ed25519"));
        config.set_ssh_agent(true);

        assert_eq!(
            config.private_key_paths(),
            &[PathBuf::from("/test/id_ed25519")]
        );
        assert!(config.ssh_agent_enabled());
    }

    #[test]
    fn test_ssh_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();

        let key_path = ssh_dir.join("id_rsa");
        fs::write(&key_path, "dummy key").unwrap();

        let config = SshConfig::new(vec![key_path], ssh_dir.join("known_hosts"), false);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ssh_config_validation_no_credentials() {
        let config = SshConfig::new(
            vec![PathBuf::from("/nonexistent/id_rsa")],
            PathBuf::from("/test/known_hosts"),
            false,
        );

        assert!(matches!(
            config.validate(),
            Err(SshError::NoCredentialsAvailable)
        ));
    }
}

