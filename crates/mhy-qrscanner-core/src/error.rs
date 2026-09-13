use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid account id")]
    InvalidAccountId,
}
