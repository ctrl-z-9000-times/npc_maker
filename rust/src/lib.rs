//!

pub mod ctrl;
pub mod env;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}
