//!

pub mod ctrl;

// mod env_api;
mod env_spec;
mod messages;

pub mod env {
    // pub use crate::env_api::{ack, death, get_args, info, mate, new, poll, score, Mode};
    pub use crate::env_spec::{EnvironmentSpec, InterfaceSpec, PopulationSpec, SettingsSpec};
    pub use crate::messages::{Request, Response};
}

/*
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{}")]
    Json(#[from] serde_json::Error),

    #[error("{}")]
    Io(#[from] std::io::Error),
}
*/
