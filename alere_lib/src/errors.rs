#[cfg(feature = "kmymoney")]
use sqlx;

#[derive(thiserror::Error, Debug)]
pub enum AlrError {
    #[error("{0}")]
    Str(String),

    #[error("{0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[cfg(feature = "kmymoney")]
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("{0}")]
    ParseError(String),
}
