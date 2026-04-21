use std::path::PathBuf;

use directories::ProjectDirs;

use crate::base::consts::*;

pub mod consts {
    /// default directory to be used by end-to-end encrypted protocol for persistent storage
    pub const STORE_DIR_DEFAULT: &str = "store/";
    /// the version number from Cargo.toml at compile time
    pub const VERSION_O: Option<&str> = option_env!("CARGO_PKG_VERSION");
    /// fallback if static compile time value is None
    pub const VERSION: &str = "unknown version";
    /// the package name from Cargo.toml at compile time, usually matrix-commander
    pub const PKG_NAME_O: Option<&str> = option_env!("CARGO_PKG_NAME");
    /// fallback if static compile time value is None
    pub const PKG_NAME: &str = "matrix-commander";
    /// the name of binary program from Cargo.toml at compile time, usually matrix-commander-rs
    pub const BIN_NAME_O: Option<&str> = option_env!("CARGO_BIN_NAME");
    /// fallback if static compile time value is None
    pub const BIN_NAME: &str = "matrix-commander-rs";
    /// fallback if static compile time value is None
    pub const BIN_NAME_UNDERSCORE: &str = "matrix_commander_rs";
    /// he repo name from Cargo.toml at compile time,
    /// e.g. string `https://github.com/8go/matrix-commander-rs/`
    pub const PKG_REPOSITORY_O: Option<&str> = option_env!("CARGO_PKG_REPOSITORY");
    /// fallback if static compile time value is None
    pub const PKG_REPOSITORY: &str = "https://github.com/8go/matrix-commander-rs/";
    /// default timeouts for waiting for the Matrix server, in seconds
    pub const TIMEOUT_DEFAULT: u64 = 60;
    /// URL for README.md file downloaded for --readme
    pub const URL_README: &str =
        "https://raw.githubusercontent.com/8go/matrix-commander-rs/main/README.md";
}

const MCRS_DEVICE_NAME: &str = "mcrs";

pub fn device_name() -> String {
    MCRS_DEVICE_NAME.to_string()
}

#[derive(thiserror::Error, Debug)]
pub enum MCRSError {
    #[error("Input/Output error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Serialization/deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Matrix error: {0}")]
    Matrix(#[from] matrix_sdk::Error),
}

/// Gets the *default* path (terminating in a directory) of the store directory
/// The default path might not be the actual path as it can be overwritten with command line
/// options.
pub fn get_store_default_path() -> PathBuf {
    let dir = ProjectDirs::from_path(PathBuf::from(get_prog_without_ext())).unwrap();
    let dp = dir.data_dir().join(STORE_DIR_DEFAULT);
    dp
}

/// Gets version number, static if available, otherwise default.
pub fn get_version() -> &'static str {
    VERSION_O.unwrap_or(VERSION)
}

/// Gets Rust package name, static if available, otherwise default.
pub fn get_pkg_name() -> &'static str {
    PKG_NAME_O.unwrap_or(PKG_NAME)
}

/// Gets Rust binary name, static if available, otherwise default.
fn get_bin_name() -> &'static str {
    BIN_NAME_O.unwrap_or(BIN_NAME)
}

/// Gets Rust package repository, static if available, otherwise default.
pub fn get_pkg_repository() -> &'static str {
    PKG_REPOSITORY_O.unwrap_or(PKG_REPOSITORY)
}

/// Gets program name without extension.
pub fn get_prog_without_ext() -> &'static str {
    get_bin_name() // with -rs suffix
                   // get_pkg_name() // without -rs suffix
}
