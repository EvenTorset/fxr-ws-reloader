use pelite::pe::Pe;
use pelite::pe::PeView;
use protocol::GameDetectionError;
use super::game_data::{self, GameData};

/// Figures out what game we're currently running inside of.
pub fn detect_running_game() -> Result<GameData, GameDetectionError> {
  let header = unsafe {
    let handle = windows::Win32::System::LibraryLoader::GetModuleHandleA(None)
      .map_err(|_| GameDetectionError::NoMainModuleHandle)?;

    PeView::module(handle.0 as *const u8)
  };

  let product_name = select_product_name(&header)?;

  // Find the game data entry that matches this window title
  game_data::get_game_data_by_title(&product_name)
    .ok_or_else(|| GameDetectionError::UnknownProductName(product_name))
}

/// Attempts to capture the product name from the PE header.
fn select_product_name(
  header: &PeView,
) -> Result<String, GameDetectionError> {
  let resources = header.resources()
    .map_err(|_| GameDetectionError::MissingPEResources)?;
  let version_info = resources.version_info()
    .map_err(|_| GameDetectionError::MissingPEVersionInfo)?;
  let language = version_info.translation().first()
    .ok_or(GameDetectionError::MissingPEStringsLanguage)?;

  let mut product_name: Option<String> = None;
  version_info.strings(*language, |k,v| if k == "ProductName" {
    product_name = Some(v.to_string())
  });

  product_name.ok_or(GameDetectionError::MissingProductName)
}
