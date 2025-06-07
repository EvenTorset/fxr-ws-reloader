# FXR WebSocket Reloader
This is a DLL mod for various From Software games that allows FXR files to be reloaded while the game is still running.

It hosts a WebSocket server that performs the reload when requested to. It can also respawn effects by modifying the game params, or list or extract loaded FXRs from the game's memory.

## Installation
You can download the mod from the [Releases page](https://github.com/EvenTorset/fxr-ws-reloader/releases/latest). To make the game load the DLL, you have three main options:

### me3
The recommended way to use the reloader is with [me3](https://github.com/garyttierney/me3/releases). If you have me3 installed, simply double click one of the me3 mod profiles included in the reloader zip file to launch the game with the reloader. If you want to use it along with other mods, open the profile in a text editor to see how the DLL is included.

### Mod Engine 2
If you use [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases/latest), you can place the DLL and its config file anywhere, and then open the config TOML file for the game you want to add it to in a text editor and add the path to the DLL file to the `external_dlls` list, like this:
```toml
external_dlls = [
  "C:\\your\\mod\\dlls\\fxr_ws_reloader.dll"
]
```

### Elden Mod Loader
If you are using [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117) to load DLL mods, simply place the DLL and its config file in your mods folder.

## Configuration
The JSON config file that comes with the DLL can be modified to change the port number used by the WebSocket server:
```jsonc
{
  "port": 24621
}
```

## Usage
The only way to control this reloader is through WebSocket requests.

### fxr-reloader library
The [fxr-reloader](https://www.npmjs.com/package/fxr-reloader) library was made to make this DLL mod easy to use from JavaScript so that scripts using [@cccode/fxr](https://www.npmjs.com/package/@cccode/fxr) to create or edit FXRs can put the FXRs directly into Elden Ring and have them respawn without manually interacting with the game at all. This allows these scripts to basically have a live preview of the effect that is being worked on.

This library isn't the only thing that can control the reloader, however. If you want to create your own client to do so, the information you need is below.

### Requests
All requests to the WebSocket server should be JSON objects that include at least two properties:
- `request_id`: A string used to identify the request. The server doesn't use this for anything. It simply includes it in the response to that request so that the client can know what request the response was for.
- `type`: The type of the request, which tells the server what to do. It has two valid values:
  - `reload_fxrs`: This patches the definitions for the given FXR files so that any new instances of it will use the new FXRs. When this request type is used, the request needs one additional property:
    - `fxrs`: An array of base64 strings of the binary data of the FXRs.
  - `set_resident_sfx`: This edits the resident SFX param fields for a given weapon based on the properties of the request. The fields are first set to `-1` and then to the given value after a very short delay, which causes the SFX to respawn. When this request type is used, the request needs three additional properties:
    - `weapon`: The numerical ID of the weapon to edit. You can find a list of these here: https://github.com/MaxTheMiracle/Dark-Souls-3-Parts-Files/blob/master/Elden%20Ring
    - `sfx`: The numerical SFX ID to change the `resident_sfx_id_1` param field to.
    - `dmy`: The numerical dummy poly ID to change the `resident_sfx_dmy_id_1` param field to.
  - `set_sp_effect_sfx`: This edits the SFX and VFX param fields for a given SpEffect based on the properties of the request. The VFX is first set to `-1` and then to the given value after a very short delay, which causes the SFX to respawn. When this request type is used, the request needs three or four additional properties:
    - `spEffect`: The numerical ID of the weapon to edit. You can find a list of these here: https://github.com/MaxTheMiracle/Dark-Souls-3-Parts-Files/blob/master/Elden%20Ring
    - `sfx`: The numerical SFX ID to change the `midst_sfx_id` param field to.
    - `dmy`: The numerical dummy poly ID to change the `midst_dmy_id` param field to.
    - `vfx`: (Optional) The numerical VFX ID to change the `vfx_id` param field to. If not given, the `vfx_id` will not be changed.
  - `get_fxr`: This will extract a loaded FXR file from the game's memory and send it back base64-encoded. When this request type is used, the request needs one additional property:
    - `id`: The ID of the FXR to extract.
  - `list_fxrs`: This will send back a list of all loaded FXR IDs.

#### Reload FXR example request payload
```json
{
  "request_id": "example_request_1",
  "type": "reload_fxrs",
  "fxrs": [
    "<base64 goes here>"
  ]
}
```

#### Set resident SFX example request payload
```json
{
  "request_id": "example_request_2",
  "type": "set_resident_sfx",
  "weapon": 24050000,
  "sfx": 402030,
  "dmy": 206
}
```

### Responses
The server will respond to all requests sent to it with a JSON object containing the request ID and a "success" property that is true only if it successfully performed the action, and message with status information.

#### Success example response
```json
{
  "request_id": "example_request_2",
  "success": true,
  "message": "Successfully set resident SFX for weapon 24050000"
}
```

#### Error example response
```json
{
  "request_id": "example_request_1",
  "success": false,
  "message": "Failed to patch FXR: example error"
}
```

#### Connection response
When a client connects to the server, it will immediately send some information about the server and its environment to the client with a special message that doesn't follow the standard response structure. This can be detected by checking if the message contains a `type` property with the value `server_info`.
```json
{
  "type": "server_info",
  "version": "3.0.0",
  "game": "Nightreign",
  "features": {
    "reload": true,
    "params": false,
    "extract": true
  }
}
```

## Credits
This reloader is built on top of [vswarte](https://github.com/vswarte)'s [fxr-reloader](https://github.com/vswarte/fxr-reloader) and [eldenring-rs](https://github.com/vswarte/eldenring-rs) projects and I could not have made this without those!
