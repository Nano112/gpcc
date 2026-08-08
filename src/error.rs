// SPDX-License-Identifier: MIT
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("decoder desynchronised: {0}")]
    Desync(String),

    #[error("invalid parameter {param}: {detail}")]
    InvalidParam { param: &'static str, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;
