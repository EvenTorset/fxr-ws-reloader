use protocol::FxrManagerError;

pub mod game;
mod singleton;

pub fn patch_fxr(fxrs: Vec<Vec<u8>>) -> Result<(), FxrManagerError> {
  let game = game::detection::detect_running_game()?;
  let patcher = game::make_fxr_manager(game)?;

  fxrs.into_iter().try_for_each(|f| patcher.patch(f))
}

pub fn extract_fxr(id: u32) -> Result<Vec<u8>, FxrManagerError> {
  let game = game::detection::detect_running_game()?;
  let patcher = game::make_fxr_manager(game)?;

  patcher.extract(id)
}

pub fn list_fxr_ids() -> Result<Vec<u32>, FxrManagerError> {
  let game = game::detection::detect_running_game()?;
  let patcher = game::make_fxr_manager(game)?;

  patcher.list_ids()
}
