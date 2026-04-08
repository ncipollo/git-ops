mod auth;
mod checkout;
mod client;
mod clone;
mod error;
mod https;
mod pull;

pub use auth::SshConfig;
pub use client::GitClient;
pub use error::{GitError, SshError};
