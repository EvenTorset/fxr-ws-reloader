#![allow(non_upper_case_globals)]

use std::ffi::c_void;
use paste::paste;
use crate::resolve_func;
use super::scanner::get_pe_view;
use protocol::FxrManagerError;
use std::collections::HashSet;
use crate::game::FxrManager;
use from_singleton::{FromSingleton, address_of};
use std::borrow::Cow;
use pelite::pattern::Atom;
use once_cell::sync::Lazy;
use std::sync::Mutex;

type FxrAllocatorGetter = unsafe extern "system" fn() -> usize;
type AllocateFxr = unsafe extern "system" fn(usize, usize, usize) -> usize;
type PatchFxrOffsets = unsafe extern "system" fn(usize, usize, usize) -> *const std::ffi::c_void;
type PrepareFxr = unsafe extern "system" fn(usize) -> *const std::ffi::c_void;

const PATCH_FXR_OFFSETS_CALL_PATTERN: &[Atom] = pelite::pattern!(
  "4c 8b 44 24 20 48 8b 54 24 20 48 8b 4c 24 20 e8 $ { ' }"
);

const PREPARE_FXR_CALL_PATTERN: &[Atom] = pelite::pattern!(
  "33 c0 e9 ? ? ? ? 48 8b 4c 24 20 e8 $ { ' }"
);

const GET_ALLOCATOR_CALL_PATTERN: &[Atom] = pelite::pattern!(
  "48 8b 44 24 28 8b 40 04 c1 e8 10 83 f8 ? ? ? 33 c0 e9 ? ? ? ? e8 $ { ' }"
);

pub unsafe extern "system" fn null_allocator() -> usize { 0 }

pub unsafe extern "system" fn null_patcher(_a: usize, _b: usize, _c: usize) -> *const c_void {
  std::ptr::null()
}

pub unsafe extern "system" fn null_preparer(_: usize) -> *const c_void {
  std::ptr::null()
}

#[derive(serde::Serialize, Debug, Clone, Copy)]
pub struct SupportedFeatures {
  pub reload: bool,
  pub params: bool,
  pub extract: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GameData {
  pub name: &'static str,
  pub product_name: &'static str,
  pub exe_names: &'static [&'static str],
  pub features: SupportedFeatures,
}

pub struct FxrDefinitionIterator {
  current: *mut FxrListNode,
}

impl Iterator for FxrDefinitionIterator {
  type Item = *mut FxrListNode;

  fn next(&mut self) -> Option<Self::Item> {
    let previous = unsafe { self.current.as_ref() }?;
    self.current = previous.next;

    let current = unsafe { self.current.as_ref() }?;
    if current.id == 0 {
      None
    } else {
      Some(self.current)
    }
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct FxrWrapper {
  fxr: usize,
  unk: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct FxrListNode {
  pub next: *mut FxrListNode,
  pub prev: *mut FxrListNode,
  pub id: u32,
  _pad14: u32,
  pub fxr_wrapper: *mut FxrWrapper,
}

// #[repr(C)]
// #[derive(Debug)]
// pub struct FxrResourceContainer {
//   pub allocator1: u64,
//   pub scene_ctrl: u64,
//   pub unk10: u64,
//   pub allocator2: u64,
//   pub fxr_list_head: *mut FxrListNode,
//   pub resource_count: u64,
// }

fn fxr_at(fxr_ptr: *const u8) -> Option<Vec<u8>> {
  unsafe {
    let version = *(fxr_ptr.add(0x6) as *const u16);
    let (ll_offset, ll_count) = if version == 5 {
      (
        *(fxr_ptr.add(0x80) as *const u32) as usize,
        *(fxr_ptr.add(0x84) as *const u32) as usize,
      )
    } else if version == 4 {
      (
        *(fxr_ptr.add(0x60) as *const u32) as usize,
        *(fxr_ptr.add(0x64) as *const u32) as usize,
      )
    } else {
      return None;
    };
    let total_size = ll_offset + (ll_count * 4);
    let mut bytes = vec![0u8; total_size];
    std::ptr::copy_nonoverlapping(fxr_ptr, bytes.as_mut_ptr(), total_size);
    Some(bytes)
  }
}

macro_rules! if_else {
  (true, $true_block:block, $false_block:block) => {
    $true_block
  };
  (false, $true_block:block, $false_block:block) => {
    $false_block
  };
}

macro_rules! define_games {
  (
    $(
      $game_ident:ident {
        product_name: $title:literal,
        exe_names: [$($exe:literal),* $(,)?],
        singleton_name: $singleton_name:literal,
        cssfx_unk_size: $cssfx_size:expr,
        gfx_manager_unk_size: $gfx_size:expr,
        res_con_pad_size: $res_con_size:expr,
        get_allocator_pattern: $get_allocator_pattern:ident,
        patch_offsets_pattern: $patch_offsets_pattern:ident,
        prepare_pattern: $prepare_pattern:ident,
        features: {
          reload: $reload:expr,
          params: $params:expr,
          extract: $extract:expr $(,)?
        } $(,)?
      }
    ),* $(,)?
  ) => {
    paste! {
      $(
        const [<$game_ident:upper _EXES>]: &[&str] = &[$($exe),*];

        pub const $game_ident: GameData = GameData {
          name: stringify!($game_ident),
          product_name: $title,
          exe_names: [<$game_ident:upper _EXES>],
          features: SupportedFeatures {
            reload: $reload,
            params: $params,
            extract: $extract,
          },
        };

        #[repr(C)]
        #[derive(Debug)]
        pub struct [<$game_ident FxrResourceContainer>] {
          pub pad: [u8; $res_con_size],
          pub fxr_list_head: *mut FxrListNode,
        }

        #[repr(C)]
        #[derive(Debug)]
        pub struct [<$game_ident GXFfxGraphicsResourceManager>] {
          pub vftable: u64,
          pub unk: [u8; $gfx_size],
          pub resource_container: &'static mut [<$game_ident FxrResourceContainer>],
        }

        #[repr(C)]
        #[derive(Debug)]
        pub struct [<$game_ident GXFfxSceneCtrl>] {
          pub vftable: u64,
          pub sg_entity: u64,
          pub allocator: u64,
          pub ffx_manager: u64,
          pub unk: u64,
          pub graphics_resource_manager: &'static mut [<$game_ident GXFfxGraphicsResourceManager>],
        }

        #[repr(C)]
        #[derive(Debug)]
        pub struct [<$game_ident CSSfx>] {
          pub vftable: u64,
          pub unk: [u8; $cssfx_size],
          pub scene_ctrl: &'static mut [<$game_ident GXFfxSceneCtrl>],
        }

        impl [<$game_ident CSSfx>] {
          pub fn fxr_definition_iter(&mut self) -> FxrDefinitionIterator {
            FxrDefinitionIterator {
              current: self
                .scene_ctrl
                .graphics_resource_manager
                .resource_container
                .fxr_list_head,
            }
          }
        }

        impl FromSingleton for [<$game_ident CSSfx>] {
          fn name() -> Cow<'static, str> {
            $singleton_name.into()
          }
        }

        pub struct [<$game_ident FxrManager>] {
          patch_fxr_offsets: PatchFxrOffsets,
          prepare_fxr: PrepareFxr,
          allocate: Box<dyn Fn(usize, usize) -> usize + Send + Sync>,
        }

        impl [<$game_ident FxrManager>] {
          pub fn new() -> Result<Self, FxrManagerError> {
            if_else! ($reload, {
              let pe = get_pe_view().unwrap();
              let get_allocator = resolve_func!("get_allocator", $get_allocator_pattern, 1, FxrAllocatorGetter, &pe);
              let allocator = unsafe { get_allocator() };
              let allocate_fn: AllocateFxr = unsafe {
                std::mem::transmute(
                  *((*(allocator as *const usize) + 0x50) as *const usize)
                )
              };
              let allocate = Box::new(move |size: usize, align: usize| unsafe {
                allocate_fn(allocator, size, align)
              });
              Ok(Self {
                patch_fxr_offsets: resolve_func!("patch_fxr_offsets", $patch_offsets_pattern, 1, PatchFxrOffsets, &pe),
                prepare_fxr: resolve_func!("prepare_fxr", $prepare_pattern, 1, PrepareFxr, &pe),
                allocate,
              })
            }, {
              Ok(Self {
                patch_fxr_offsets: null_patcher,
                prepare_fxr: null_preparer,
                allocate: Box::new(|_, _| 0),
              })
            })
          }
        }

        impl FxrManager for [<$game_ident FxrManager>] {
          fn patch(&self, fxr_bytes: Vec<u8>) -> Result<(), FxrManagerError> {
            if_else! ($extract, {
              if fxr_bytes.len() < 0x10 {
                return Err(FxrManagerError::InvalidFxr);
              }

              let fxr_id = u32::from_le_bytes(
                fxr_bytes[0xc..0x10]
                  .try_into()
                  .map_err(|_| FxrManagerError::InvalidFxr)?,
              );

              let sfx_imp = unsafe {
                address_of::<[<$game_ident CSSfx>]>()
                  .ok_or(FxrManagerError::CSSfxInstanceMissing)?
                  .as_mut()
              };

              let fxr = sfx_imp
                .fxr_definition_iter()
                .filter_map(|f| unsafe { f.as_mut() })
                .find(|f| f.id == fxr_id);

              if let Some(fxr) = fxr {
                unsafe {
                  let allocation = (self.allocate)(fxr_bytes.len(), 0x10);
                  std::ptr::copy_nonoverlapping(
                    fxr_bytes.as_ptr(),
                    allocation as *mut u8,
                    fxr_bytes.len(),
                  );

                  (self.patch_fxr_offsets)(allocation, allocation, allocation);
                  (self.prepare_fxr)(allocation);

                  if let Some(wrapper) = fxr.fxr_wrapper.as_mut() {
                    wrapper.fxr = allocation;
                  }
                }
              }

              Ok(())
            }, {
              Err(FxrManagerError::UnsupportedOperation(
                format!("FXR reloading is not supported in {}", stringify!($game_ident))
              ))
            })
          }

          fn extract(&self, fxr_id: u32) -> Result<Vec<u8>, FxrManagerError> {
            if_else! ($extract, {
              let sfx_imp = unsafe {
                address_of::<[<$game_ident CSSfx>]>()
                  .ok_or(FxrManagerError::CSSfxInstanceMissing)?
                  .as_mut()
              };

              let fxr = sfx_imp
                .fxr_definition_iter()
                .filter_map(|f| unsafe { f.as_mut() })
                .find(|f| f.id == fxr_id)
                .ok_or(FxrManagerError::FxrNotFound(fxr_id))?;

              unsafe {
                if let Some(wrapper) = fxr.fxr_wrapper.as_mut() {
                  let fxr_ptr = wrapper.fxr as *const u8;
                  return fxr_at(fxr_ptr).ok_or(FxrManagerError::InvalidFxr);
                }
              }

              Err(FxrManagerError::FxrNotFound(fxr_id))
            }, {
              Err(FxrManagerError::UnsupportedOperation(
                format!("FXR extraction is not supported in {}", stringify!($game_ident))
              ))
            })
          }

          fn extract_multiple(&self, fxr_ids: &Vec<u32>) -> Result<Vec<Option<Vec<u8>>>, FxrManagerError> {
            if_else! ($extract, {
              let sfx_imp = unsafe {
                address_of::<[<$game_ident CSSfx>]>()
                  .ok_or(FxrManagerError::CSSfxInstanceMissing)?
                  .as_mut()
              };

              let mut fxr_map = std::collections::HashMap::new();
              sfx_imp
                .fxr_definition_iter()
                .filter_map(|f| unsafe { f.as_mut() })
                .for_each(|fxr| {
                  fxr_map.insert(fxr.id, fxr);
                });

              let mut result = Vec::with_capacity(fxr_ids.len());
              for id in fxr_ids {
                match fxr_map.get(id) {
                  Some(fxr) => unsafe {
                    if let Some(wrapper) = (*fxr).fxr_wrapper.as_mut() {
                      let fxr_ptr = wrapper.fxr as *const u8;
                      result.push(fxr_at(fxr_ptr));
                    } else {
                      result.push(None);
                    }
                  },
                  None => {
                    result.push(None);
                  }
                }
              }

              Ok(result)
            }, {
              Err(FxrManagerError::UnsupportedOperation(
                format!("FXR extraction is not supported in {}", stringify!($game_ident))
              ))
            })
          }

          fn list_ids(&self) -> Result<Vec<u32>, FxrManagerError> {
            if_else! ($extract, {
              let sfx_imp = unsafe {
                address_of::<[<$game_ident CSSfx>]>()
                  .ok_or(FxrManagerError::CSSfxInstanceMissing)?
                  .as_mut()
              };

              let ids: Vec<u32> = sfx_imp
                .fxr_definition_iter()
                .filter_map(|f| unsafe { f.as_mut() })
                .map(|f| f.id)
                .collect();

              Ok(ids)
            }, {
              Err(FxrManagerError::UnsupportedOperation(
                format!("FXR listing is not supported in {}", stringify!($game_ident))
              ))
            })
          }
        }
      )*

      pub const SUPPORTED_GAMES: &[GameData] = &[
        $($game_ident),*
      ];

      static FXR_MANAGER_CACHE: Lazy<Mutex<Option<&'static (dyn FxrManager + Send + Sync)>>> = Lazy::new(|| Mutex::new(None));

      pub(crate) fn fxr_manager_for(game: &GameData) -> Result<&'static (dyn FxrManager + Send + Sync), FxrManagerError> {
        let mut cache = FXR_MANAGER_CACHE.lock().unwrap();
        if let Some(manager) = *cache {
          return Ok(manager);
        }

        let manager: Box<dyn FxrManager + Send + Sync> = match game.name {
          $(
            stringify!($game_ident) => Box::new([<$game_ident FxrManager>]::new()?),
          )*
          _ => return Err(FxrManagerError::UnsupportedGame),
        };
        let static_manager: &'static (dyn FxrManager + Send + Sync) = Box::leak(manager);
        *cache = Some(static_manager);
        Ok(static_manager)
      }
    }
  };
}

define_games! {
  DarkSouls3 {
    product_name: "DARK SOULS™ III",
    exe_names: ["DarkSoulsIII.exe"],
    singleton_name: "SprjSfx",
    cssfx_unk_size: 0x50,
    gfx_manager_unk_size: 0x158,
    res_con_pad_size: 0x10,
    get_allocator_pattern: GET_ALLOCATOR_CALL_PATTERN,
    patch_offsets_pattern: PATCH_FXR_OFFSETS_CALL_PATTERN,
    prepare_pattern: PREPARE_FXR_CALL_PATTERN,
    features: {
      reload: true,
      params: false,
      extract: true
    },
  },
  Sekiro {
    product_name: "Sekiro™: Shadows Die Twice",
    exe_names: ["sekiro.exe"],
    singleton_name: "SprjSfx",
    cssfx_unk_size: 0x58,
    gfx_manager_unk_size: 0x158,
    res_con_pad_size: 0x20,
    get_allocator_pattern: GET_ALLOCATOR_CALL_PATTERN,
    patch_offsets_pattern: PATCH_FXR_OFFSETS_CALL_PATTERN,
    prepare_pattern: PREPARE_FXR_CALL_PATTERN,
    features: {
      reload: true,
      params: false,
      extract: true
    },
  },
  EldenRing {
    product_name: "ELDEN RING™",
    exe_names: ["eldenring.exe", "start_protected_game.exe"],
    singleton_name: "CSSfx",
    cssfx_unk_size: 0x58,
    gfx_manager_unk_size: 0x158,
    res_con_pad_size: 0x20,
    get_allocator_pattern: GET_ALLOCATOR_CALL_PATTERN,
    patch_offsets_pattern: PATCH_FXR_OFFSETS_CALL_PATTERN,
    prepare_pattern: PREPARE_FXR_CALL_PATTERN,
    features: {
      reload: true,
      params: true,
      extract: true
    },
  },
  ArmoredCore6 {
    product_name: "ARMORED CORE™ VI FIRES OF RUBICON™",
    exe_names: ["armoredcore6.exe", "start_protected_game.exe"],
    singleton_name: "CSSfx",
    cssfx_unk_size: 0x88,
    gfx_manager_unk_size: 0x58,
    res_con_pad_size: 0x20,
    get_allocator_pattern: GET_ALLOCATOR_CALL_PATTERN,
    patch_offsets_pattern: PATCH_FXR_OFFSETS_CALL_PATTERN,
    prepare_pattern: PREPARE_FXR_CALL_PATTERN,
    features: {
      reload: true,
      params: false,
      extract: true
    },
  },
  Nightreign {
    product_name: "ELDEN RING NIGHTREIGN",
    exe_names: ["nightreign.exe", "start_protected_game.exe"],
    singleton_name: "CSSfx",
    cssfx_unk_size: 0x58,
    gfx_manager_unk_size: 0x58,
    res_con_pad_size: 0x20,
    get_allocator_pattern: GET_ALLOCATOR_CALL_PATTERN,
    patch_offsets_pattern: PATCH_FXR_OFFSETS_CALL_PATTERN,
    prepare_pattern: PREPARE_FXR_CALL_PATTERN,
    features: {
      reload: true,
      params: false,
      extract: true
    },
  },
}

pub fn get_game_data_by_title(product_name: &str) -> Option<GameData> {
  SUPPORTED_GAMES.iter()
    .find(|game| game.product_name == product_name)
    .copied()
}

pub fn get_supported_exe_names() -> Vec<&'static str> {
  let mut seen = HashSet::new();
  SUPPORTED_GAMES
    .iter()
    .flat_map(|g| g.exe_names.iter().copied())
    .filter(|exe| seen.insert(*exe))
    .collect()
}
