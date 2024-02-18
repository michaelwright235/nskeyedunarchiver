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
