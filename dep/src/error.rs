use thiserror::Error;
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("IO error {0}")]
    IO(#[from] std::io::Error),
    #[error("Iced error {0}")]
    Iced(#[from] iced::Error),
    #[error("Parse Int error {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("from utf8 error {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("unknown data store error")]
    Unknown,
    // #[error("Missing values in parameter: {0}")]
    // MissingValues(String),
}

#[derive(Debug, Clone)]
pub enum LoadError {
    FileError,
    FormatError,
}

#[derive(Debug, Clone)]
pub enum CMDError {
    ExcError,
}

#[derive(Debug, Clone)]
enum Error {
    APIError,
    LanguageError,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        dbg!(error);

        Error::APIError
    }
}
