use crate::auth::CredentialCallback;
use crate::error::GitError;

/// Check if a URL uses HTTPS protocol
pub(crate) fn is_https_url(url: &str) -> bool {
    url.starts_with("https://")
}

/// Check if HTTPS credentials are available via credential helper or environment variables
pub(crate) fn has_https_credentials() -> bool {
    if let Ok(config) = get_git_config() {
        if config.get_string("credential.helper").is_ok() {
            return true;
        }
    }
    std::env::var("GITHUB_TOKEN").is_ok()
        || std::env::var("GH_TOKEN").is_ok()
        || std::env::var("GITHUB_ACCESS_TOKEN").is_ok()
}

/// Get git config (quiet, no warnings)
fn get_git_config() -> Result<git2::Config, git2::Error> {
    git2::Config::open_default().or_else(|_| git2::Config::new())
}

/// Get git config, warning if no credential helpers are configured
pub(crate) fn get_git_config_with_credential_helpers() -> Result<git2::Config, git2::Error> {
    let config = git2::Config::open_default().or_else(|_| git2::Config::new())?;

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
pub(crate) fn https_credentials_callback() -> Result<CredentialCallback, GitError> {
    Ok(Box::new(
        |url: &str, username_from_url: Option<&str>, allowed_types: git2::CredentialType| {
            // Try git credential helper first
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                if let Ok(config) = get_git_config_with_credential_helpers() {
                    if let Ok(cred) = git2::Cred::credential_helper(&config, url, username_from_url)
                    {
                        return Ok(cred);
                    }
                }
            }

            // Fallback to environment variables for backward compatibility
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                    let username = username_from_url.unwrap_or("git");
                    if let Ok(cred) = git2::Cred::userpass_plaintext(username, &token) {
                        return Ok(cred);
                    }
                }

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

            Err(git2::Error::from_str(
                "No HTTPS credentials found. Configure git credential helper or set GITHUB_TOKEN environment variable for private repositories.",
            ))
        },
    ))
}
