use crate::{ARCHIVER, ARCHIVER_VERSION};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    PlistError(#[from] plist::Error),
    #[error("Incorrect NSKeyedArchive format: {0}")]
    IncorrectFormat(String),
    #[error("Missing object #{0}")]
    MissingObject(u64),
    #[error("Missing key '{0}' for object '{1}'")]
    MissingObjectKey(String, String),
    #[error("Wrong object value type. Expected '{0}' for key '{1}'")]
    WrongObjectValueType(String, String),
    #[error("{0}")]
    DecodingObjectError(String),
}

impl From<IncorrectFormatError> for Error {
    fn from(value: IncorrectFormatError) -> Self {
        Self::IncorrectFormat(value.to_string())
    }
}

#[derive(Error, Debug)]
pub(crate) enum IncorrectFormatError {
    #[error("Expected '{0}' key to be a type of '{1}'")]
    WrongValueType(&'static str, &'static str),
    #[error("Missing '{0}' header key")]
    MissingHeaderKey(&'static str),
    #[error("Unsupported archiver. Only '{ARCHIVER}' is supported")]
    UnsupportedArchiver,
    #[error("Unsupported archiver version. Only '{ARCHIVER_VERSION}' is supported")]
    UnsupportedArchiverVersion,
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
    #[error("{0}")] // "Missing key '{0}' for object '{1}'"
    MissingObjectKey(String),
    #[error("{0}")] // Incorrect value type of {0} for object {1}. Expected '{2}' for key '{3}'
    IncorrectObjectValueType(String),
}
