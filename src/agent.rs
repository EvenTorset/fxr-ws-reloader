use std::path::PathBuf;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use windows::Win32::Foundation::BOOL;
use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use base64::{engine::general_purpose, Engine as _};
use eldenring::fd4::FD4ParamRepository;
use eldenring_util::singleton;
use patcher::game::game_data::GameData;

static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();
static PARAM_REQ_CHANNEL: OnceCell<(mpsc::Sender<ParamsRequestType>, mpsc::Receiver<Response>)> = OnceCell::new();
static GAME_DATA: OnceCell<GameData> = OnceCell::new();

#[derive(Deserialize, Debug)]
struct Config {
  port: u16,
  console: bool,
}

#[derive(Deserialize, Debug)]
enum RequestType {
  #[serde(rename = "reload_fxrs")]
  ReloadFXRs,
  #[serde(rename = "set_resident_sfx")]
  SetResidentSFX,
  #[serde(rename = "set_sp_effect_sfx")]
  SetSpEffectSFX,
  #[serde(rename = "get_fxr")]
  GetFXR,
  #[serde(rename = "get_fxrs")]
  GetFXRs,
  #[serde(rename = "list_fxrs")]
  ListFXRs,
  #[serde(other)]
  Unknown,
}

const REQUEST_TYPE_NAMES: &[&str] = &[
  "reload_fxrs",
  "set_resident_sfx",
  "set_sp_effect_sfx",
  "get_fxr",
  "get_fxrs",
  "list_fxrs",
];

impl Default for RequestType {
  fn default() -> Self {
    RequestType::Unknown
  }
}

#[derive(Deserialize, Debug)]
struct Request {
  request_id: String,
  #[serde(rename = "type", default)]
  request_type: RequestType,
  #[serde(flatten, default)]
  params: serde_json::Value,
}

#[derive(Serialize)]
struct Response {
  request_id: String,
  success: bool,
  message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  data: Option<serde_json::Value>,
}

#[derive(Clone)]
enum ParamsRequestType {
  SetResidentSFX { weapon_id: u32, sfx_id: i32, dmy_id: i32 },
  SetSpEffectSFX { sp_effect_id: u32, sfx_id: i32, dmy_id: i16, target_vfx_id: Option<i32> },
}

fn get_dll_dir_path() -> Option<PathBuf> {
  let dll_name = "fxr_ws_reloader.dll\0";
  let wide_dll_name: Vec<u16> = dll_name.encode_utf16().collect();
  let module = unsafe { GetModuleHandleW(PCWSTR::from_raw(wide_dll_name.as_ptr())) }.ok()?;
  let mut buffer = [0u16; 260];
  let length = unsafe { GetModuleFileNameW(module, &mut buffer) };
  if length == 0 {
    return None;
  }

  let path_str = String::from_utf16_lossy(&buffer[..length as usize]);
  let path = PathBuf::from(path_str);
  Some(path.parent()?.to_path_buf())
}

#[no_mangle]
pub extern "system" fn DllMain(
  _inst: isize,
  reason: u32,
  _: *mut std::ffi::c_void,
) -> BOOL {
  if reason == 1 { // DLL_PROCESS_ATTACH
    std::thread::spawn(|| {
      let runtime = tokio::runtime::Runtime::new().unwrap();

      let game_data = match patcher::game::detection::detect_running_game() {
        Ok(data) => data,
        Err(e) => {
          eprintln!("Failed to detect a supported game: {}", e);
          return;
        }
      };
      if GAME_DATA.set(game_data).is_err() {
        eprintln!("Failed to set GAME_DATA");
        return;
      }

      let config: Config = {
        let config_path = get_dll_dir_path()
          .map(|p| p.join("fxr_ws_reloader_config.json"))
          .unwrap_or_else(|| PathBuf::from("fxr_ws_reloader_config.json"));

        let config_str = std::fs::read_to_string(config_path)
          .unwrap_or_else(|_| String::from(r#"{"port": 24621}"#));
        serde_json::from_str(&config_str).unwrap_or(Config { port: 24621, console: false })
      };

      if config.console {
        unsafe {
          windows::Win32::System::Console::AllocConsole();
        }
      }

      let (tx, rx) = mpsc::channel::<ParamsRequestType>(32);
      let (response_tx, response_rx) = mpsc::channel::<Response>(32);
      PARAM_REQ_CHANNEL.set((tx.clone(), response_rx)).unwrap();

      runtime.spawn(game_param_handler(rx, response_tx));

      runtime.spawn(async move {
        start_websocket_server(config.port).await;
      });

      RUNTIME.set(runtime).unwrap();
    });
  } else if reason == 0 { // DLL_PROCESS_DETACH
    if let Some(_) = RUNTIME.get() {
      // Runtime will be dropped automatically when DLL is unloaded
    }
  }
  BOOL(1)
}

async fn start_websocket_server(port: u16) {
  let addr = format!("127.0.0.1:{}", port);
  let listener = TcpListener::bind(&addr).await.unwrap();
  let game_info = GAME_DATA.get()
    .map(|g| format!(" | Game: {}", g.name))
    .unwrap_or_else(|| " | Game: ERROR: Unsupported or undetected game".to_string());
  println!(
    "WebSocket server listening on: {} | Version: {}{}",
    addr,
    env!("CARGO_PKG_VERSION"),
    game_info
  );

  while let Ok((stream, _)) = listener.accept().await {
    tokio::spawn(handle_connection(stream));
  }
}

async fn handle_connection(stream: TcpStream) {
  if let Ok(addr) = stream.peer_addr() {
    println!("New WebSocket connection from {}", addr);
  } else {
    println!("New WebSocket connection from unknown address");
  }
  let ws_stream = accept_async(stream).await.unwrap();
  let (mut write, mut read) = ws_stream.split();

  let game_data = match GAME_DATA.get() {
    Some(data) => data.clone(),
    None => {
      let response = serde_json::json!({
        "type": "server_info",
        "version": env!("CARGO_PKG_VERSION"),
        "error": "Failed to detect a supported game."
      });
      if let Err(e) = write.send(Message::Text(response.to_string())).await {
        eprintln!("Failed to send error message: {}", e);
        return;
      }
      return;
    }
  };

  let server_info = serde_json::json!({
    "type": "server_info",
    "version": env!("CARGO_PKG_VERSION"),
    "game": game_data.name,
    "features": game_data.features
  });
  if let Err(e) = write.send(Message::Text(server_info.to_string())).await {
    eprintln!("Failed to send server info: {}", e);
    return;
  }

  let params_sender = PARAM_REQ_CHANNEL.get().unwrap().0.clone();
  let (response_tx, mut response_rx) = mpsc::channel::<(String, Response)>(32);
  let write_handle = tokio::spawn(async move {
    while let Some((id, response)) = response_rx.recv().await {
      let response_text = serde_json::to_string(&response).unwrap();
      if let Err(e) = write.send(Message::Text(response_text)).await {
        eprintln!("Error sending response for request {}: {}", id, e);
        break;
      }
    }
  });

  while let Some(msg) = read.next().await {
    if let Ok(msg) = msg {
      if let Message::Text(text) = msg {
        let request = match serde_json::from_str::<Request>(&text) {
          Ok(request) => request,
          Err(e) => {
            let response = Response {
              request_id: ":ERROR:".into(),
              success: false,
              message: format!("Invalid request format: {}", e),
              data: None,
            };
            if let Err(e) = response_tx.send(("error".to_string(), response)).await {
              eprintln!("Error sending error response: {}", e);
              break;
            }
            continue;
          }
        };

        let request_id = request.request_id.clone();
        let response_tx = response_tx.clone();
        let params_sender = params_sender.clone();
        let game_data = game_data.clone();

        match request.request_type {
          RequestType::SetResidentSFX | RequestType::SetSpEffectSFX | RequestType::ReloadFXRs => {
            let response = handle_request(request, params_sender, game_data).await;
            if let Err(e) = response_tx.send((request_id, response)).await {
              eprintln!("Error sending response: {}", e);
              break;
            }
          },
          _ => {
            tokio::spawn(async move {
              let response = handle_request(request, params_sender, game_data).await;
              if let Err(e) = response_tx.send((request_id, response)).await {
                eprintln!("Error sending response: {}", e);
              }
            });
          }
        }
      }
    }
  }

  drop(response_tx);
  let _ = write_handle.await;
}

async fn handle_request(
  request: Request,
  params_sender: mpsc::Sender<ParamsRequestType>,
  game_data: GameData
) -> Response {
  match request.request_type {
    RequestType::ReloadFXRs => {
      if !game_data.features.reload {
        eprintln!("FXR reloading is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("FXR reloading is not supported in {}", game_data.name),
          data: None,
        };
      }
      if let Some(fxrs) = request.params.get("fxrs") {
        if let Some(fxr_array) = fxrs.as_array() {
          let mut fxr_bytes: Vec<Vec<u8>> = Vec::new();
          for fxr in fxr_array {
            if let Some(base64_str) = fxr.as_str() {
              match general_purpose::STANDARD.decode(base64_str) {
                Ok(bytes) => fxr_bytes.push(bytes),
                Err(e) => {
                  eprintln!("Failed to decode base64 FXR: {}", e);
                  return Response {
                    request_id: request.request_id,
                    success: false,
                    message: format!("Failed to decode base64 FXR: {}", e),
                    data: None,
                  }
                }
              }
            }
          }
          let fxr_id_opt = if fxr_bytes.len() == 1 && fxr_bytes[0].len() >= 0x10 {
            Some(u32::from_le_bytes(fxr_bytes[0][0xc..0x10].try_into().unwrap()))
          } else {
            None
          };
          match patcher::patch(&game_data, fxr_bytes) {
            Ok(_) => {
              if let Some(id) = fxr_id_opt {
                println!("Reloaded FXR {}", id);
              } else {
                println!("Reloaded {} FXRs", fxr_array.len());
              }
              Response {
                request_id: request.request_id,
                success: true,
                message: "Successfully reloaded FXR".to_string(),
                data: None,
              }
            },
            Err(e) => {
              eprintln!("Failed to patch FXR: {}", e);
              Response {
                request_id: request.request_id,
                success: false,
                message: format!("Failed to patch FXR: {}", e),
                data: None,
              }
            }
          }
        } else {
          eprintln!("Invalid fxrs parameter: expected array");
          Response {
            request_id: request.request_id,
            success: false,
            message: "Invalid fxrs parameter: expected array".to_string(),
            data: None,
          }
        }
      } else {
        eprintln!("Missing fxrs parameter");
        Response {
          request_id: request.request_id,
          success: false,
          message: "Missing fxrs parameter".to_string(),
          data: None,
        }
      }
    }
    RequestType::SetResidentSFX => {
      if !game_data.features.params {
        eprintln!("Parameter modification is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("Parameter modification is not supported in {}", game_data.name),
          data: None,
        };
      }
      let weapon_id = match request.params.get("weapon").and_then(|v| v.as_u64()) {
        Some(id) => id as u32,
        None => {
          eprintln!("Missing or invalid weapon parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid weapon parameter".to_string(),
            data: None,
          }
        }
      };

      let sfx_id = match request.params.get("sfx").and_then(|v| v.as_u64()) {
        Some(id) => id as i32,
        None => {
          eprintln!("Missing or invalid sfx parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid sfx parameter".to_string(),
            data: None,
          }
        }
      };

      let dmy_id = match request.params.get("dmy").and_then(|v| v.as_u64()) {
        Some(id) => id as i32,
        None => {
          eprintln!("Missing or invalid dmy parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid dmy parameter".to_string(),
            data: None,
          }
        }
      };

      if let Err(e) = params_sender.send(ParamsRequestType::SetResidentSFX { weapon_id, sfx_id, dmy_id }).await {
        eprintln!("Failed to send params request: {}", e);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("Failed to send params request: {}", e),
          data: None,
        };
      }

      println!("Set resident SFX: weapon_id={}, sfx_id={}, dmy_id={}", weapon_id, sfx_id, dmy_id);
      Response {
        request_id: request.request_id,
        success: true,
        message: format!("Successfully set resident SFX for weapon {}", weapon_id),
        data: None,
      }
    }
    RequestType::SetSpEffectSFX => {
      if !game_data.features.params {
        eprintln!("Parameter modification is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("Parameter modification is not supported in {}", game_data.name),
          data: None,
        };
      }
      let sp_effect_id = match request.params.get("spEffect").and_then(|v| v.as_u64()) {
        Some(id) => id as u32,
        None => {
          eprintln!("Missing or invalid spEffect parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid spEffect parameter".to_string(),
            data: None,
          }
        }
      };

      let sfx_id = match request.params.get("sfx").and_then(|v| v.as_u64()) {
        Some(id) => id as i32,
        None => {
          eprintln!("Missing or invalid sfx parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid sfx parameter".to_string(),
            data: None,
          }
        }
      };

      let dmy_id = match request.params.get("dmy").and_then(|v| v.as_u64()) {
        Some(id) => id as i16,
        None => {
          eprintln!("Missing or invalid dmy parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid dmy parameter".to_string(),
            data: None,
          }
        }
      };

      let target_vfx_id = request.params.get("vfx").and_then(|v| v.as_i64()).map(|id| id as i32);

      if let Err(e) = params_sender.send(ParamsRequestType::SetSpEffectSFX { 
        sp_effect_id, 
        sfx_id, 
        dmy_id, 
        target_vfx_id 
      }).await {
        eprintln!("Failed to send params request: {}", e);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("Failed to send params request: {}", e),
          data: None,
        };
      }

      println!("Set SpEffect SFX: sp_effect_id={}, sfx_id={}, dmy_id={}, target_vfx_id={:?}", sp_effect_id, sfx_id, dmy_id, target_vfx_id);
      Response {
        request_id: request.request_id,
        success: true,
        message: format!("Successfully updated SFX for SpEffect {}", sp_effect_id),
        data: None,
      }
    }
    RequestType::GetFXR => {
      if !game_data.features.extract {
        eprintln!("FXR extraction is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("FXR extraction is not supported in {}", game_data.name),
          data: None,
        };
      }
      let fxr_id = match request.params.get("id").and_then(|v| v.as_u64()) {
        Some(id) => id as u32,
        None => {
          eprintln!("Missing or invalid id parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid id parameter".to_string(),
            data: None,
          }
        }
      };

      let fxr_bytes = match patcher::extract(&game_data, fxr_id) {
        Ok(bytes) => bytes,
        Err(e) => {
          eprintln!("Failed to extract FXR: {}", e);
          return Response {
            request_id: request.request_id,
            success: false,
            message: format!("Failed to extract FXR: {}", e),
            data: None,
          }
        }
      };

      let base64_str = general_purpose::STANDARD.encode(&fxr_bytes);
      println!("Extracted FXR {}", fxr_id);
      Response {
        request_id: request.request_id,
        success: true,
        message: "Successfully extracted FXR".to_string(),
        data: Some(serde_json::json!({ "fxr": base64_str })),
      }
    }
    RequestType::GetFXRs => {
      if !game_data.features.extract {
        eprintln!("FXR extraction is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("FXR extraction is not supported in {}", game_data.name),
          data: None,
        };
      }
      let ids = match request.params.get("ids").and_then(|v| v.as_array()) {
        Some(ids) => ids.iter()
          .filter_map(|v| v.as_u64().map(|id| id as u32))
          .collect::<Vec<_>>(),
        None => {
          eprintln!("Missing or invalid ids parameter");
          return Response {
            request_id: request.request_id,
            success: false,
            message: "Missing or invalid ids parameter".to_string(),
            data: None,
          }
        }
      };

      let fxrs = match patcher::extract_multiple(&game_data, &ids) {
        Ok(bytes_vec) => bytes_vec,
        Err(e) => {
          eprintln!("Failed to extract FXRs: {}", e);
          return Response {
            request_id: request.request_id,
            success: false,
            message: format!("Failed to extract FXRs: {}", e),
            data: None,
          }
        }
      };

      let base64_fxrs: Vec<Option<String>> = fxrs.into_iter()
        .map(|maybe_bytes| maybe_bytes.map(|bytes| general_purpose::STANDARD.encode(&bytes)))
        .collect();

      if ids.len() == 1 {
        println!("Extracted FXR {}", ids[0]);
      } else {
        println!("Extracted {} FXRs", base64_fxrs.len());
      }
      Response {
        request_id: request.request_id,
        success: true,
        message: "Successfully extracted FXRs".to_string(),
        data: Some(serde_json::json!({ "fxrs": base64_fxrs })),
      }
    }
    RequestType::ListFXRs => {
      if !game_data.features.extract {
        eprintln!("FXR listing is not supported in {}", game_data.name);
        return Response {
          request_id: request.request_id,
          success: false,
          message: format!("FXR listing is not supported in {}", game_data.name),
          data: None,
        };
      }
      let fxr_ids = match patcher::list_ids(&game_data) {
        Ok(ids) => ids,
        Err(e) => {
          eprintln!("Failed to list FXRs: {}", e);
          return Response {
            request_id: request.request_id,
            success: false,
            message: format!("Failed to list FXRs: {}", e),
            data: None,
          }
        }
      };

      println!("Listed {} FXR(s)", fxr_ids.len());
      Response {
        request_id: request.request_id,
        success: true,
        message: "Successfully listed FXRs".to_string(),
        data: Some(serde_json::json!({ "fxrs": fxr_ids })),
      }
    }
    RequestType::Unknown => {
      eprintln!("Invalid request type. Valid types are: {}", REQUEST_TYPE_NAMES.join(", "));
      Response {
        request_id: request.request_id,
        success: false,
        message: format!("Invalid request type. Valid types are: {}", REQUEST_TYPE_NAMES.join(", ")),
        data: None,
      }
    }
  }
}

async fn game_param_handler(mut rx: mpsc::Receiver<ParamsRequestType>, _tx: mpsc::Sender<Response>) {
  while let Some(request) = rx.recv().await {
    match request {
      ParamsRequestType::SetResidentSFX { weapon_id, sfx_id, dmy_id } => {
        let param_repo = unsafe { singleton::get_instance::<FD4ParamRepository>() }
          .unwrap_or_else(|_| panic!("Could not get reflection data for FD4ParamRepository"));

        if let Some(instance) = param_repo {
          if let Some(weapon_row) = (*instance).get_mut::<eldenring::param::EQUIP_PARAM_WEAPON_ST>(weapon_id) {
            weapon_row.set_resident_sfx_id_1(-1);
            weapon_row.set_resident_sfx_dmy_id_1(-1);
          }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let param_repo = unsafe { singleton::get_instance::<FD4ParamRepository>() }
          .unwrap_or_else(|_| panic!("Could not get reflection data for FD4ParamRepository"));

        if let Some(instance) = param_repo {
          if let Some(weapon_row) = (*instance).get_mut::<eldenring::param::EQUIP_PARAM_WEAPON_ST>(weapon_id) {
            weapon_row.set_resident_sfx_id_1(sfx_id);
            weapon_row.set_resident_sfx_dmy_id_1(dmy_id);
          }
        }
      }
      ParamsRequestType::SetSpEffectSFX { sp_effect_id, sfx_id, dmy_id, target_vfx_id } => {
        let param_repo = unsafe { singleton::get_instance::<FD4ParamRepository>() }
          .unwrap_or_else(|_| panic!("Could not get reflection data for FD4ParamRepository"));

        let vfx_id = if let Some(instance) = param_repo {
          if let Some(sp_effect_row) = (*instance).get_mut::<eldenring::param::SP_EFFECT_PARAM_ST>(sp_effect_id) {
            let current_vfx_id = sp_effect_row.vfx_id();
            sp_effect_row.set_vfx_id(-1);
            sp_effect_row.set_vfx_id1(-1);
            target_vfx_id.unwrap_or(current_vfx_id)
          } else {
            -1
          }
        } else {
          -1
        };

        if vfx_id != -1 {
          let param_repo = unsafe { singleton::get_instance::<FD4ParamRepository>() }
            .unwrap_or_else(|_| panic!("Could not get reflection data for FD4ParamRepository"));

          if let Some(instance) = param_repo {
            if let Some(vfx_row) = (*instance).get_mut::<eldenring::param::SP_EFFECT_VFX_PARAM_ST>(vfx_id as u32) {
              vfx_row.set_midst_sfx_id(sfx_id);
              vfx_row.set_midst_dmy_id(dmy_id);
            }
          }

          tokio::time::sleep(std::time::Duration::from_millis(100)).await;

          let param_repo = unsafe { singleton::get_instance::<FD4ParamRepository>() }
            .unwrap_or_else(|_| panic!("Could not get reflection data for FD4ParamRepository"));

          if let Some(instance) = param_repo {
            if let Some(sp_effect_row) = (*instance).get_mut::<eldenring::param::SP_EFFECT_PARAM_ST>(sp_effect_id) {
              sp_effect_row.set_vfx_id(vfx_id);
            }
          }
        }
      }
    }
  }
}
