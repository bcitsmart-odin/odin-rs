use thiserror::Error;

pub type Result<T> = std::result::Result<T, BcitSmartError>;

#[derive(Error,Debug)]
pub enum BcitSmartError {

    #[error("build error {0}")]
    BuildError( #[from] odin_build::OdinBuildError),

    #[error("IO error {0}")]
    IOError( #[from] std::io::Error),

    #[error("time delta out of range error {0}")]
    DurationError( #[from] chrono::OutOfRangeError),

    #[error("No object error")]
    NoObjectError( String ),

    #[error("No object key error")]
    NoObjectKeyError(),

    #[error("NetCDF data set error: {0}")]
    DatasetError( String ),

    #[error("No object date error")]
    NoObjectDateError(),

    #[error("String to float conversion error {0}")]
    FloatConversionError( #[from] std::num::ParseFloatError),

    #[error("invalid filename")]
    FilenameError(String),

    #[error("Misc error {0}")]
    MiscError( String ),

    #[error("serde error {0}")]
    SerdeError( #[from] serde_json::Error),

    #[error("ODIN Actor error {0}")]
    OdinActorError( #[from] odin_actor::errors::OdinActorError),

    #[error("ODIN GDAL error {0}")]
    OdinGdalError( #[from] odin_gdal::errors::OdinGdalError),

    #[error("ODIN GDAL error {0}")]
    GdalError( #[from] odin_gdal::errors::GdalError),

    #[error("UTF-8 conversion error {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("SQLx error {0}")]
    SqlxError(#[from] sqlx::Error),
}

pub fn misc_error (msg: impl ToString)->BcitSmartError {
    BcitSmartError::MiscError(msg.to_string())
}

pub fn no_object_error (msg: impl ToString)->BcitSmartError {
    BcitSmartError::NoObjectError(msg.to_string())
}

pub fn filename_error (msg: impl ToString)->BcitSmartError {
    BcitSmartError::FilenameError(msg.to_string())
}