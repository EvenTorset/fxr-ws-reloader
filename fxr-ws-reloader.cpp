#include "base.hpp"

#include <Windows.h>

#include <iostream>
#include <fstream>

#if defined(INCLUDE_LIBER) && INCLUDE_LIBER == 0
  #undef INCLUDE_LIBER
#endif

#ifdef INCLUDE_LIBER
  #include <dantelion2/system.hpp>
  #include <detail/windows.inl>
  #include <coresystem/file/file.hpp>
  #include <coresystem/cs_param.hpp>
  #include <param/param.hpp>
#endif

#include <websocketpp/config/asio_no_tls.hpp>
#include <websocketpp/server.hpp>
#include <websocketpp/base64/base64.hpp>

#include <nlohmann/json.hpp>
using json = nlohmann::json;

extern "C" {
  void patch_fxr(const char* process_name, const unsigned char* fxr_bytes, size_t fxr_size);
}

typedef websocketpp::server<websocketpp::config::asio> server;

server ws_server;

const std::string LOG_PREFIX = "[fxr-ws-reloader] ";

std::string getDLLDirPath() {
  HMODULE hModule = GetModuleHandle("fxr-ws-reloader.dll");
  if (hModule) {
    char buffer[MAX_PATH];
    DWORD length = GetModuleFileName(hModule, buffer, MAX_PATH);
    if (length > 0) {
      std::string fullPath(buffer);
      size_t lastSlashPos = fullPath.find_last_of("\\/");
      if (lastSlashPos != std::string::npos) {
        return fullPath.substr(0, lastSlashPos);
      }
    }
  }
  return "";
}

std::string getEXEName() {
  HMODULE hModule = GetModuleHandle(nullptr);
  if (hModule) {
    char buffer[MAX_PATH];
    DWORD length = GetModuleFileName(hModule, buffer, MAX_PATH);
    if (length > 0) {
      std::string fullPath(buffer);
      size_t lastSlashPos = fullPath.find_last_of("\\/");
      if (lastSlashPos != std::string::npos) {
        return fullPath.substr(lastSlashPos + 1);
      }
    }
  }
  return "unknown";
}

enum RequestType {
  ReloadFXR = 0,
  SetResidentSFX = 1,
};

void respond(websocketpp::connection_hdl hdl, json req, std::string status) {
  json res {
    { "requestID", req["requestID"] },
    { "status", status },
  };
  ws_server.send(hdl, res.dump(), websocketpp::frame::opcode::text);
}

void on_message(websocketpp::connection_hdl hdl, server::message_ptr msg) {
  json req = json::parse(msg->get_payload());
  if (!req.contains("requestID")) {
    json res {
      { "status", "Missing request ID" },
    };
    ws_server.send(hdl, res.dump(), websocketpp::frame::opcode::text);
    return;
  }
  if (!req.contains("type")) {
    respond(hdl, req, "Missing request type");
    return;
  }
  int req_type = req["type"];
  switch (req_type) {
    case RequestType::ReloadFXR: {
      std::string binaryData = websocketpp::base64_decode(req["file"]);
      const unsigned char* bdc = reinterpret_cast<const unsigned char*>(binaryData.c_str());
      if (bdc[0] != 0x46 || bdc[1] != 0x58 || bdc[2] != 0x52 || bdc[3] != 0) {
        std::cout << LOG_PREFIX << "Requested reload of invalid FXR" << '\n';
        respond(hdl, req, "Invalid FXR");
      }
      uint32_t fxr_id = (bdc[15] << 24) | (bdc[14] << 16) | (bdc[13] << 8) | bdc[12];
      patch_fxr(getEXEName().c_str(), bdc, binaryData.size());
      std::cout << LOG_PREFIX << "Reloaded FXR: " << fxr_id << '\n';
      respond(hdl, req, "success");
      break;
    }
    #ifdef INCLUDE_LIBER
      case RequestType::SetResidentSFX: {
        from::CS::SoloParamRepository::wait_for_params(-1);
        int weaponID = req["weapon"];
        auto [row, row_exists] = from::param::EquipParamWeapon[weaponID];
        if (row_exists) {
          int sfxID = req["sfx"];
          int dmyID = req["dmy"];
          row.residentSfxId_1 = sfxID;
          row.residentSfx_DmyId_1 = dmyID;
          std::cout << LOG_PREFIX <<
            "Edited weapon resident sfx and dmy ID: Weapon: " << weaponID <<
            ", sfx: " << sfxID <<
            ", dmy: " << dmyID << '\n';
          respond(hdl, req, "success");
        } else {
          std::cout << LOG_PREFIX << "Weapon not found:" << weaponID << '\n';
          respond(hdl, req, "Weapon not found");
        }
        break;
      }
    #else
      case RequestType::SetResidentSFX: {
        respond(hdl, req, "Param editing requires libER");
        break;
      }
    #endif
    default:
      std::cout << LOG_PREFIX << "Unrecognized request type: " << req["type"] << '\n';
      respond(hdl, req, "Unrecognized request type");
      break;
  }
}

void reloader_main() {
  std::ifstream f(getDLLDirPath() + "\\fxr-ws-reloader-config.json");
  json config = json::parse(f, nullptr, true, true);

  if (config["log"]) {
    con_allocate(false);
  }

  #ifdef INCLUDE_LIBER
    from::DLSY::wait_for_system(-1);
  #endif

  ws_server.set_message_handler(&on_message);
  ws_server.set_access_channels(websocketpp::log::alevel::none);
  ws_server.set_error_channels(websocketpp::log::elevel::none);

  ws_server.init_asio();
  ws_server.listen(config["port"]);
  ws_server.start_accept();

  std::cout << LOG_PREFIX << "WebSocket server listening on port " << config["port"] << '\n';
  ws_server.run();
}
