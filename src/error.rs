use thiserror::Error;

pub(crate) type KotlinLSPWrapperResult<T> =
  std::result::Result<T, KotlinLSPWrapperError>;

#[derive(Debug, Error)]
pub(crate) enum KotlinLSPWrapperError {
  #[error("IoError. Error: {0}")]
  Io(#[from] std::io::Error),
}
