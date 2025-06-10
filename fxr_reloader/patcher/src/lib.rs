use protocol::FxrManagerError;
use game::game_data::GameData;

pub mod game;

pub fn patch(game_data: &GameData, fxrs: Vec<Vec<u8>>) -> Result<(), FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  fxrs.into_iter().try_for_each(|f| manager.patch(f))
}

pub fn extract(game_data: &GameData, id: u32) -> Result<Vec<u8>, FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  manager.extract(id)
}

pub fn extract_multiple(game_data: &GameData, ids: &Vec<u32>) -> Result<Vec<Option<Vec<u8>>>, FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  manager.extract_multiple(ids)
}

pub fn list_ids(game_data: &GameData) -> Result<Vec<u32>, FxrManagerError> {
  let manager = game::game_data::fxr_manager_for(game_data)?;

  manager.list_ids()
}
