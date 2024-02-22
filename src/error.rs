use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    PlistError(#[from] plist::Error),
    #[error("Incorrect NSKeyedArchive format: {0}")]
    IncorrectFormat(String),
}

#[derive(Error, Debug)]
pub enum DeError {
    #[error("{0}")]
    Message(String),
    #[error("Expected string")]
    ExpectedString,
    #[error("Expected integer")]
    ExpectedInteger,
    #[error("Expected real")]
    ExpectedReal,
    #[error("Expected boolean")]
    ExpectedBoolean,
    #[error("Expected data")]
    ExpectedData,
    #[error("Expected object")]
    ExpectedObject,
    #[error("Expected null reference")]
    ExpectedNullRef,
}
