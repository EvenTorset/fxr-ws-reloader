use protocol::PatchFxrError;

pub mod game;
mod singleton;

pub fn patch_fxr(fxrs: Vec<Vec<u8>>) -> Result<(), PatchFxrError> {
    let game = game::detection::detect_running_game()?;
    let patcher = game::make_patcher(game)?;

    fxrs.into_iter().try_for_each(|f| patcher.patch(f))
}
