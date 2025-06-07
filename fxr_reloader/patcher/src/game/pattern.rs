use std::collections::HashMap;
use std::sync::Mutex;
use std::mem;
use std::ops;
use std::ffi;

use windows::core::PCSTR;
use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;
use windows::Win32::System::Diagnostics::Debug::{
  ImageNtHeader,
  IMAGE_NT_HEADERS64,
  IMAGE_SECTION_HEADER,
};

use once_cell::sync::Lazy;
use broadsword::scanner;

static PATTERN_CACHE: Lazy<Mutex<HashMap<String, Option<PatternResult>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub enum LookupError {
  ModuleNotFound,
  SectionNotFound,
}

pub fn get_module_handle(module: impl AsRef<str>) -> Result<usize, LookupError> {
  unsafe {
    GetModuleHandleW(string_to_pcwstr(module))
      .map_err(|_| LookupError::ModuleNotFound)
      .map(|x| x.0 as usize)
  }
}

fn string_to_pcwstr(input: impl AsRef<str>) -> PCWSTR {
  PCWSTR::from_raw([
    input.as_ref()
      .encode_utf16()
      .collect::<Vec<u16>>(),
      vec![0x0_u16]
  ].concat().as_ptr())
}

/// Retrieves all address ranges of sections in a module matching the given name.
pub fn get_all_module_section_ranges(
  module: impl AsRef<str>,
  specified_section: impl AsRef<str>
) -> Result<Vec<ops::Range<usize>>, LookupError> {
  let module_base = get_module_handle(module)?;

  let image_nt_header = unsafe { ImageNtHeader(module_base as *const ffi::c_void) };
  let num_sections = unsafe { (*image_nt_header).FileHeader.NumberOfSections as u32 };
  let number_of_rva_and_sizes =
    unsafe { (*image_nt_header).OptionalHeader.NumberOfRvaAndSizes };

  let dos_header = module_base as *const IMAGE_DOS_HEADER;
  let nt_header_base = module_base + unsafe { (*dos_header).e_lfanew as usize };

  let section_base = nt_header_base
    + mem::size_of::<IMAGE_NT_HEADERS64>()
    + ((number_of_rva_and_sizes - 16) * 8) as usize;

  let specified_section = specified_section.as_ref();
  let mut results = Vec::new();

  unsafe {
    let mut current_section_header = section_base;
    let section_header_size = mem::size_of::<IMAGE_SECTION_HEADER>();

    for _ in 0..num_sections {
      let section_header = current_section_header as *const IMAGE_SECTION_HEADER;

      let section_name = PCSTR::from_raw((*section_header).Name.as_ptr())
        .to_string()
        .unwrap_or_default();

      if section_name == specified_section {
        let section_size = (*section_header).Misc.VirtualSize;
        let section_va = (*section_header).VirtualAddress;

        let start = module_base + section_va as usize;
        let end = start + section_size as usize;
        results.push(ops::Range { start, end });
      }

      current_section_header += section_header_size;
    }
  }

  if results.is_empty() {
    Err(LookupError::SectionNotFound)
  } else {
    Ok(results)
  }
}

/// Takes an instruction pattern and looks for its location
pub(crate) fn match_instruction_pattern(pattern: &str) -> Option<PatternResult> {
  let mut cache = PATTERN_CACHE.lock().unwrap();
  if let Some(cached) = cache.get(pattern) {
    return cached.clone();
  }

  let result = {
    let text_sections: Vec<_> = super::game_data::get_supported_exe_names()
      .into_iter()
      .filter_map(|exe| get_all_module_section_ranges(exe, ".text").ok())
      .flatten()
      .collect();

    let pattern = scanner::Pattern::from_bit_pattern(pattern).unwrap();

    text_sections.into_iter().find_map(|text_section| {
      let scan_slice = unsafe {
        std::slice::from_raw_parts(
          text_section.start as *const u8,
          text_section.end - text_section.start,
        )
      };

      scanner::simple::scan(scan_slice, &pattern).map(|result| PatternResult {
        location: text_section.start + result.location,
        captures: result.captures.into_iter()
          .map(|capture| PatternCapture {
            location: text_section.start + capture.location,
            bytes: capture.bytes,
          })
          .collect(),
      })
    })
  };

  cache.insert(pattern.to_string(), result.clone());
  result
}

#[derive(Debug, Clone)]
pub(crate) struct PatternResult {
  pub location: usize,
  pub captures: Vec<PatternCapture>,
}

#[derive(Debug, Clone)]
pub(crate) struct PatternCapture {
  pub location: usize,
  pub bytes: Vec<u8>,
}

// pub(crate) const GET_ALLOCATOR_PATTERN_DS3: &str = concat!(
//   "01001... 10001011 01000100 ..100100 00101000",
//   "10001011 01000000 00000100",
//   "11000001 11101000 00010000",
//   "10000011 11111000 00000100",
//   "01110110 ........",
//   "00110011 11000000",
//   "11101001 ........ ........ ........ ........",
//   "11101000 [........ ........ ........ ........]",
// );

// 1420fbf20 4c 89 44        MOV        qword ptr [RSP + 0x18]=>local_res18,R8
//           24 18
// 1420fbf25 48 89 54        MOV        qword ptr [RSP + 0x10]=>local_res10,RDX
//           24 10
// 1420fbf2a 48 89 4c        MOV        qword ptr [RSP + 0x8]=>local_res8,RCX
//           24 08
// 1420fbf2f 57              PUSH       RDI
// 1420fbf30 48 81 ec        SUB        RSP,0x100
//           00 01 00 00
// 1420fbf37 48 8b fc        MOV        RDI,RSP
// 1420fbf3a b9 40 00        MOV        ECX,0x40
//           00 00
// 1420fbf3f b8 cc cc        MOV        EAX,0xcccccccc
//           cc cc
// 1420fbf44 f3 ab           STOSD.REP  RDI
// 1420fbf46 48 8b 8c        MOV        RCX,qword ptr [RSP + 0x110]=>local_res8
//           24 10 01 
//           00 00
// 1420fbf4e 48 8b 84        MOV        RAX,qword ptr [RSP + 0x110]=>local_res8
//           24 10 01 
//           00 00
pub(crate) const PATCH_OFFSETS_PATTERN: &str = concat!(
  "01001... 10001001 01000100 ..100100 00011000",
  "01001... 10001001 01010100 ..100100 00010000",
  "01001... 10001001 01001100 ..100100 00001000",
  "01010111",
  "01001... 10000001 11101100 00000000 00000001 00000000 00000000",
  "01001... 10001011 11111100",
  "10111001 01000000 00000000 00000000 00000000",
  "10111000 11001100 11001100 11001100 11001100",
  "11110011 10101011",
  "01001... 10001011 10001100 ..100100 00010000 00000001 00000000 00000000",
  "01001... 10001011 10000100 ..100100 00010000 00000001 00000000 00000000",
);

// 142125030 48 89 4c        MOV        qword ptr [RSP + 0x8]=>local_res8,RCX
//           24 08
// 142125035 57              PUSH       RDI
// 142125036 48 81 ec        SUB        RSP,0x130
//           30 01 00 00
// 14212503d 48 8b fc        MOV        RDI,RSP
// 142125040 b9 4c 00        MOV        ECX,0x4c
//           00 00
// 142125045 b8 cc cc        MOV        EAX,0xcccccccc
//           cc cc
// 14212504a f3 ab           STOSD.REP  RDI
// 14212504c 48 8b 8c        MOV        RCX,qword ptr [RSP + 0x140]=>local_res8
//           24 40 01 
//           00 00
// 142125054 48 8b 84        MOV        RAX,qword ptr [RSP + 0x140]=>local_res8
//           24 40 01 
//           00 00
pub(crate) const WTF_FXR_PATTERN: &str = concat!(
  "01001... 10001001 01001100 ..100100 00001000",
  "01010111",
  "01001... 10000001 11101100 00110000 00000001 00000000 00000000",
  "01001... 10001011 11111100",
  "10111001 01001100 00000000 00000000 00000000",
  "10111000 11001100 11001100 11001100 11001100",
  "11110011 10101011",
  "01001... 10001011 10001100 ..100100 01000000 00000001 00000000 00000000",
  "01001... 10001011 10000100 ..100100 01000000 00000001 00000000 00000000",
);

// 1420fbda7 48 8b 44        MOV        RAX,qword ptr [RSP + 0x28]=>local_50
//           24 28
// 1420fbdac 8b 40 04        MOV        EAX,dword ptr [RAX + 0x4]
// 1420fbdaf c1 e8 10        SHR        EAX,0x10
// 1420fbdb2 83 f8 05        CMP        EAX,0x5
// 1420fbdb5 74 07           JZ         LAB_1420fbdbe
// 1420fbdb7 33 c0           XOR        EAX,EAX
// 1420fbdb9 e9 59 01        JMP        LAB_1420fbf17
//           00 00
// 1420fbdbe e8 cd bb        CALL       FUN_1420b7990
//           fb ff
pub(crate) const GET_ALLOCATOR_PATTERN: &str = concat!(
  "01001... 10001011 01000100 ..100100 00101000",
  "10001011 01000000 00000100",
  "11000001 11101000 00010000",
  "10000011 11111000 00000101",
  "01110100 ........",
  "00110011 11000000",
  "11101001 ........ ........ ........ ........",
  "11101000 [........ ........ ........ ........]",
);

// 1422bf3b0 4c 89 44        MOV        qword ptr [RSP + 0x18],R8
//           24 18
// 1422bf3b5 48 89 54        MOV        qword ptr [RSP + 0x10],RDX
//           24 10
// 1422bf3ba 48 89 4c        MOV        qword ptr [RSP + 0x8],RCX
//           24 08
// 1422bf3bf 57              PUSH       RDI
// 1422bf3c0 48 81 ec        SUB        RSP,0x100
//           00 01 00 00
// 1422bf3c7 48 8b 84        MOV        RAX,qword ptr [RSP + 0x110]
//           24 10 01 
//           00 00
// 1422bf3cf 48 89 04 24     MOV        qword ptr [RSP],RAX
// 1422bf3d3 48 8b 84        MOV        RAX,qword ptr [RSP + 0x118]
//           24 18 01 
//           00 00
pub(crate) const PATCH_OFFSETS_PATTERN_NR: &str = concat!(
  "01001... 10001001 01000100 ..100100 00011000",
  "01001... 10001001 01010100 ..100100 00010000",
  "01001... 10001001 01001100 ..100100 00001000",
  "01010111",
  "01001... 10000001 11101100 00000000 00000001 00000000 00000000",
  "01001... 10001011 10000100 ..100100 00010000 00000001 00000000 00000000",
  "01001... 10001001 00000100 ..100100",
  "01001... 10001011 10000100 ..100100 00011000 00000001 00000000 00000000"
);

// 146336401 48 89 4c        MOV        qword ptr [RSP + 0x8],RCX
//           24 08
// 146336406 48 8d 64        LEA        RSP,[RSP + -0x8]
//           24 f8
// 14633640b 48 89 3c 24     MOV        qword ptr [RSP],RDI
// 14633640f 48 81 ec        SUB        RSP,0x130
//           30 01 00 00
// 146336416 48 8d 7c        LEA        RDI,[RSP + 0x20]
//           24 20
// 14633641b b9 44 00        MOV        ECX,0x44
//           00 00
// 146336420 b8 cc cc        MOV        EAX,0xcccccccc
//           cc cc
// 146336425 f3 ab           STOSD.REP  RDI
// 146336427 48 8b 8c        MOV        RCX,qword ptr [RSP + 0x140]
//           24 40 01 
//           00 00
// 14633642f 48 8b 84        MOV        RAX,qword ptr [RSP + 0x140]
//           24 40 01 
//           00 00
pub(crate) const WTF_FXR_PATTERN_NR: &str = concat!(
  "01001... 10001001 01001100 ..100100 00001000",
  "01001... 10001101 01100100 ..100100 11111000",
  "01001... 10001001 00111100 ..100100",
  "01001... 10000001 11101100 00110000 00000001 00000000 00000000",
  "01001... 10001101 01111100 ..100100 00100000",
  "10111001 01000100 00000000 00000000 00000000",
  "10111000 11001100 11001100 11001100 11001100",
  "11110011 10101011",
  "01001... 10001011 10001100 ..100100 01000000 00000001 00000000 00000000",
  "01001... 10001011 10000100 ..100100 01000000 00000001 00000000 00000000"
);
