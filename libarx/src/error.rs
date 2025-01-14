use thiserror::Error;

use crate::common::EntryType;

#[derive(Error, Debug)]
#[error("Path {0} not found in archive")]
pub struct PathNotFound(pub crate::PathBuf);

#[derive(Error, Debug)]
#[error("Jbk archive is not a valid Arx archive : {0}")]
pub struct ArxFormatError(pub &'static str);

#[derive(Error, Debug)]
#[error("Arx entry ({actual}) is not of the expected type ({expected}).")]
pub struct WrongType {
    pub expected: EntryType,
    pub actual: EntryType,
}

#[derive(Error, Debug)]
#[error("Incoherent structure : {0}")]
pub struct IncoherentStructure(pub String);

#[derive(Error, Debug)]
#[error("Invalid input : {0}")]
pub struct InputError(pub String);

#[derive(Error, Debug)]
pub enum BaseError {
    #[error(transparent)]
    Jbk(#[from] jbk::Error),
    #[error(transparent)]
    ArxFormatError(#[from] ArxFormatError),
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error(transparent)]
    BaseError(#[from] BaseError),
    #[error(transparent)]
    PathNotFound(#[from] PathNotFound),
}

impl From<jbk::Error> for QueryError {
    fn from(value: jbk::Error) -> Self {
        Self::BaseError(value.into())
    }
}

#[derive(Error, Debug)]
pub enum ArxError {
    #[error(transparent)]
    BaseError(#[from] BaseError),

    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    PathNotFound(#[from] PathNotFound),

    #[error("Not a directory")]
    NotADirectory,
    #[error("Is a directory")]
    IsADirectory,
    #[error("Is a link")]
    IsALink,
}

impl From<jbk::Error> for ArxError {
    fn from(value: jbk::Error) -> Self {
        Self::BaseError(value.into())
    }
}

impl From<ArxFormatError> for ArxError {
    fn from(value: ArxFormatError) -> Self {
        Self::BaseError(value.into())
    }
}

impl From<QueryError> for ArxError {
    fn from(value: QueryError) -> Self {
        match value {
            QueryError::BaseError(e) => Self::BaseError(e),
            QueryError::PathNotFound(e) => Self::PathNotFound(e),
        }
    }
}

#[derive(Error, Debug)]
pub enum MountError {
    #[error(transparent)]
    ArxError(#[from] ArxError),

    #[error("Cannot mount Read/Write")]
    CannotMountRW,
}

#[derive(Error, Debug)]
pub enum FsError {
    #[error(transparent)]
    BaseError(#[from] BaseError),

    #[error(transparent)]
    WrongType(#[from] WrongType),

    #[error("Not found")]
    NotFound,
    #[error("Missing Pack")]
    MissingPack,
}

impl From<jbk::Error> for FsError {
    fn from(value: jbk::Error) -> Self {
        Self::BaseError(value.into())
    }
}

impl From<ArxFormatError> for FsError {
    fn from(value: ArxFormatError) -> Self {
        Self::BaseError(value.into())
    }
}

// TODO: Move in arx crate
#[derive(Error, Debug)]
pub enum ExtractError {
    #[error(transparent)]
    ArxError(#[from] ArxError),

    #[error("File {path} already exists.", path = path.display())]
    FileExists { path: std::path::PathBuf },
}

impl From<jbk::Error> for ExtractError {
    fn from(value: jbk::Error) -> Self {
        Self::ArxError(value.into())
    }
}
impl From<ArxFormatError> for ExtractError {
    fn from(value: ArxFormatError) -> Self {
        Self::ArxError(value.into())
    }
}
impl From<BaseError> for ExtractError {
    fn from(value: BaseError) -> Self {
        Self::ArxError(value.into())
    }
}
impl From<std::io::Error> for ExtractError {
    fn from(value: std::io::Error) -> Self {
        Self::ArxError(value.into())
    }
}

#[derive(Error, Debug)]
pub enum CreatorError {
    #[error(transparent)]
    Jbk(#[from] jbk::creator::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    IncoherentStructure(#[from] IncoherentStructure),
    #[error(transparent)]
    InputError(#[from] InputError),
}
