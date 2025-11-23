mod auth;
mod checkout;
mod client;
mod error;
mod pull;

pub use auth::SshConfig;
pub use client::GitClient;
pub use error::{GitError, SshError};
