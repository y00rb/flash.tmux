pub mod config;
pub mod input;
pub mod model;
pub mod render;
pub mod search;
pub mod session;
pub mod terminal;
pub mod tmux;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub mod syntax;
