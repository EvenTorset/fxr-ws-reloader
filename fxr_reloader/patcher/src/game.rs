use protocol::FxrManagerError;

pub(crate) mod scanner;
pub mod detection;
pub mod game_data;

pub(crate) trait FxrManager {
  fn patch(&self, fxr: Vec<u8>) -> Result<(), FxrManagerError>;
  fn extract(&self, fxr_id: u32) -> Result<Vec<u8>, FxrManagerError>;
  fn extract_multiple(&self, fxr_ids: &Vec<u32>) -> Result<Vec<Option<Vec<u8>>>, FxrManagerError>;
  fn list_ids(&self) -> Result<Vec<u32>, FxrManagerError>;
}
