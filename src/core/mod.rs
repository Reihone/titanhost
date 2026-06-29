pub mod backup;
pub mod config_manager;
pub mod downloader;
pub mod error;
pub mod pinger;
pub mod process;
pub mod zerotier;

pub use error::AppError;

/// Base directory for all Minecraft server files, runtimes, configurations, and scripts
pub const SERVER_DIR: &str = "./run";
