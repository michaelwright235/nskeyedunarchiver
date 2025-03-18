use thiserror::Error;

/// An error that can happen during parsing an archive.
#[derive(Error, Debug)]
pub enum Error {
    /// Happens if something went wrong during opening or parsing a plist.
    #[error(transparent)]
    PlistError(#[from] plist::Error),

    /// Happens if an archive itself has an incorrent structure.
    #[error("Incorrect NSKeyedArchive format: {0}")]
    IncorrectFormat(String),
}

/// An error that may happen during decoding an [Object](crate::Object).
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DeError {
    #[error("{0}")]
    Custom(String),
    #[error("Expected string")]
    ExpectedString,
    #[error("Expected integer")]
    ExpectedInteger,
    #[error("Expected float")]
    ExpectedFloat,
    #[error("Expected boolean")]
    ExpectedBoolean,
    #[error("Expected data")]
    ExpectedData,
    #[error("Expected object")]
    ExpectedObject,
    #[error("Expected null reference")]
    ExpectedNullRef,
    #[error("{0}: Missing object key `{1}`")]
    MissingObjectKey(String, String),
    #[error("Expected class `{1}`, found `{0}`")]
    UnexpectedClass(String, String)
}
