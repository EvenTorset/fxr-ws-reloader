use windows::Win32::System::LibraryLoader::GetModuleHandleA;

use pelite::pe::{Pe, PeView};
use pelite::pattern::Atom;

pub fn get_pe_view() -> Result<PeView<'static>, &'static str> {
  unsafe {
    let handle = match GetModuleHandleA(None) {
      Ok(h) => h.0 as *const u8,
      Err(_) => return Err("Failed to get module handle"),
    };
    Ok(PeView::module(handle))
  }
}

pub fn resolve_pattern_va(
  pe: &PeView,
  pattern: &[Atom],
  capture_index: usize,
) -> Option<u64> {
  let mut matches = vec![0u32; capture_index + 1];
  if !pe.scanner().finds_code(pattern, &mut matches) {
    return None;
  }

  matches.get(capture_index).and_then(|rva| pe.rva_to_va(*rva).ok())
}

#[macro_export]
macro_rules! resolve_func {
  (
    $fn_name:literal,
    $pattern:expr,
    $capture_index:expr,
    $fn_ty:ty,
    $pe:expr
  ) => {{
    let va = $crate::game::scanner::resolve_pattern_va($pe, $pattern, $capture_index)
      .expect(&format!("Failed to resolve function: {}", $fn_name));
    unsafe { std::mem::transmute::<u64, $fn_ty>(va) }
  }};
}
