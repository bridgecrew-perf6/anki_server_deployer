use thiserror::Error;
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("IO error {0}")]
    IO(#[from] std::io::Error),
    #[error("Parse Int error {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("from utf8 error {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("unknown data store error")]
    Unknown,

}
