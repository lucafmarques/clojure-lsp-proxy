use thiserror::Error;

pub(crate) type ClojureLspProxyResult<T> =
  std::result::Result<T, ClojureLspProxyError>;

#[derive(Debug, Error)]
pub(crate) enum ClojureLspProxyError {
  #[error("IoError. Error: {0}")]
  Io(#[from] std::io::Error),

  #[error("ParseIntError. Error: {0}")]
  ParseInt(#[from] std::num::ParseIntError),

  #[error("SerdeError. Error: {0}")]
  Serde(#[from] serde_json::Error),

  #[error("ZipError. Error: {0}")]
  Zip(#[from] zip::result::ZipError),

  #[error("GeneralError. Error `{0}`")]
  General(String),
}
