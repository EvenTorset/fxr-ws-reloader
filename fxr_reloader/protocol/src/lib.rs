use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum FxrManagerError {
  #[error("Could not locate CSSfx instance.")]
  CSSfxInstanceMissing,
  #[error("Failed parsing the supplier FXR. Can't read FXR ID.")]
  InvalidFxr,
  #[error("Could not acquire game parameters: {0}")]
  GameDetectionError(#[from] GameDetectionError),
  #[error("Could not find FXR with ID {0}.")]
  FxrNotFound(u32),
  #[error("{0}")]
  UnsupportedOperation(String),
  #[error("Unsupported game.")]
  UnsupportedGame,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum GameDetectionError {
  #[error("Failed acquiring the games module handle.")]
  NoMainModuleHandle,
  #[error("Failed acquiring PE resources.")]
  MissingPEResources,
  #[error("Failed acquiring PE version info.")]
  MissingPEVersionInfo,
  #[error("Failed acquiring PE language for strings.")]
  MissingPEStringsLanguage,
  #[error("Failed acquiring product name from PE header. Cannot determine what game we're running.")]
  MissingProductName,
  #[error("Did not recognize game for product name {0}.")]
  UnknownProductName(String),
  #[error("Failed to find the .text section.")]
  MissingTextSection,
}
