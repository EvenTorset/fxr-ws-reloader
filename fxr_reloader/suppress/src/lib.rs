use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::Mutex;
use std::ptr::null_mut;

use winapi::um::handleapi::{CloseHandle, DuplicateHandle};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::processenv::SetStdHandle;
use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
use winapi::um::winbase::{STD_OUTPUT_HANDLE, STD_ERROR_HANDLE};
use winapi::um::winnt::{
  FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_ATTRIBUTE_NORMAL, GENERIC_WRITE, HANDLE,
};

lazy_static::lazy_static! {
  static ref IO_LOCK: Mutex<()> = Mutex::new(());
}

fn get_null_handle() -> HANDLE {
  let nul: Vec<u16> = OsStr::new("NUL").encode_wide().chain(std::iter::once(0)).collect();

  unsafe {
    CreateFileW(
      nul.as_ptr(),
      GENERIC_WRITE,
      FILE_SHARE_READ | FILE_SHARE_WRITE,
      null_mut(),
      OPEN_EXISTING,
      FILE_ATTRIBUTE_NORMAL,
      null_mut(),
    )
  }
}

pub fn suppress_output<F: FnOnce() -> T, T>(f: F) -> T {
  let _guard = IO_LOCK.lock().unwrap();

  unsafe {
    let proc = GetCurrentProcess();
    let null_handle = get_null_handle();

    let mut saved_out: HANDLE = null_mut();
    let mut saved_err: HANDLE = null_mut();

    let stdout = winapi::um::processenv::GetStdHandle(STD_OUTPUT_HANDLE);
    let stderr = winapi::um::processenv::GetStdHandle(STD_ERROR_HANDLE);

    DuplicateHandle(proc, stdout, proc, &mut saved_out, 0, 1, 0);
    DuplicateHandle(proc, stderr, proc, &mut saved_err, 0, 1, 0);

    SetStdHandle(STD_OUTPUT_HANDLE, null_handle);
    SetStdHandle(STD_ERROR_HANDLE, null_handle);

    let result = f();

    SetStdHandle(STD_OUTPUT_HANDLE, saved_out);
    SetStdHandle(STD_ERROR_HANDLE, saved_err);

    CloseHandle(null_handle);

    result
  }
}
