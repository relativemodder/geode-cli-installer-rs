use std::io;
use colored::Colorize;

#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    #[error("Invalid choice. Please try again.")]
    NotANumber,

    #[error("Invalid input. Please enter a number.")]
    InvalidNumber,

    #[error("Failed to initialize installer: {0}")]
    Init(String),

    #[error("Installation failed: {0}")]
    Installation(String),

    #[error("An error occurred: {0}")]
    Unknown(String),
}

impl InstallerError {
    pub fn format(&self) -> String {
        format!("❌ {}", self).red().bold().to_string()
    }
}

impl From<io::Error> for InstallerError {
    fn from(e: io::Error) -> Self {
        InstallerError::Unknown(e.to_string())
    }
}

impl From<reqwest::Error> for InstallerError {
    fn from(e: reqwest::Error) -> Self {
        InstallerError::Unknown(e.to_string())
    }
}

impl From<serde_json::Error> for InstallerError {
    fn from(e: serde_json::Error) -> Self {
        InstallerError::Unknown(e.to_string())
    }
}

impl From<zip::result::ZipError> for InstallerError {
    fn from(e: zip::result::ZipError) -> Self {
        InstallerError::Unknown(format!("Zip error: {}", e))
    }
}

impl From<String> for InstallerError {
    fn from(err: String) -> Self {
        InstallerError::Installation(err)
    }
}
