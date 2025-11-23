/// Type alias for credential callback function used in Git operations
pub type CredentialCallback =
    Box<dyn FnMut(&str, Option<&str>, git2::CredentialType) -> Result<git2::Cred, git2::Error>>;

