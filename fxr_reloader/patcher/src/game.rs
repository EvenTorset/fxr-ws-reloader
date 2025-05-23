use protocol::FxrManagerError;
use detection::RunningGame;
use eldenring::EldenRingFxrManager;
use armoredcore6::ArmoredCore6FxrManager;

pub(crate) mod pattern;
pub mod detection;
pub(crate) mod eldenring;
pub(crate) mod armoredcore6;

pub(crate) fn make_fxr_manager(game: RunningGame) -> Result<Box<dyn FxrManager>, FxrManagerError> {
  Ok(match game {
    RunningGame::EldenRing => Box::new(EldenRingFxrManager::new()?),
    RunningGame::ArmoredCore6 => Box::new(ArmoredCore6FxrManager::new()?),
  })
}

pub(crate) trait FxrManager {
  fn patch(&self, fxr: Vec<u8>) -> Result<(), FxrManagerError>;
  fn extract(&self, fxr_id: u32) -> Result<Vec<u8>, FxrManagerError>;
  fn list_ids(&self) -> Result<Vec<u32>, FxrManagerError>;
}
