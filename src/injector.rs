use dll_syringe::{Syringe, process::OwnedProcess};
use std::{env, path::PathBuf, process};

fn main() {
  let dll_path = get_dll_path()
    .expect("Failed to find DLL path. Make sure fxr_ws_reloader.dll is in the same directory as the injector and that it has the correct name.");

  let supported_exes = patcher::game::game_data::get_supported_exe_names();
  let target_process = supported_exes.iter()
    .find_map(|&exe_name| OwnedProcess::find_first_by_name(exe_name));

  let process = match target_process {
    Some(p) => p,
    None => {
      eprintln!("No supported game process found. Supported games:");
      for exe in supported_exes {
        eprintln!("- {}", exe);
      }
      process::exit(1);
    }
  };
  println!("Found supported game process");

  let syringe = Syringe::for_process(process);
  println!("Injecting DLL: {}", dll_path.display());
  match syringe.inject(dll_path) {
    Ok(_) => {
      println!("DLL injection successful!");
      process::exit(0);
    }
    Err(e) => {
      eprintln!("Failed to inject DLL: {}", e);
      process::exit(1);
    }
  }
}

fn get_dll_path() -> Option<PathBuf> {
  let exe_path = env::current_exe().ok()?;
  let dll_name = "fxr_ws_reloader.dll";

  let dll_path = exe_path.parent()?.join(dll_name);
  if dll_path.exists() {
    return Some(dll_path);
  }

  None
}
