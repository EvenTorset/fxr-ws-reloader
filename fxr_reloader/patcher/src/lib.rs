use protocol::FxrManagerError;
use game::game_data::GameData;

pub mod game;
mod singleton;

pub fn patch_fxr(game_data: &GameData, fxrs: Vec<Vec<u8>>) -> Result<(), FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  fxrs.into_iter().try_for_each(|f| manager.patch(f))
}

pub fn extract_fxr(game_data: &GameData, id: u32) -> Result<Vec<u8>, FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  manager.extract(id)
}

pub fn list_fxr_ids(game_data: &GameData) -> Result<Vec<u32>, FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  manager.list_ids()
}
