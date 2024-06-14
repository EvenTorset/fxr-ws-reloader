# FXR WebSocket Reloader
This is a DLL mod for Elden Ring that allows FXR files to be reloaded while the game is still running.

It hosts a WebSocket server that performs the reload when requested to. It can also be requested to change the resident SFX param fields of a weapon, which can be used to respawn SFX.

## Installation
You can download the mod from the [Releases page](https://github.com/EvenTorset/fxr-ws-reloader/releases/latest). To make the game load the DLL, you have two main options:

### Mod Engine 2
If you use [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases/latest), you can place the DLL and its config file anywhere, and then open the `config_eldenring.toml` file in a text editor and add the path to the DLL file to the `external_dlls` list, like this:
```toml
external_dlls = [
  "C:\\your\\elden_ring\\dlls\\fxr-ws-reloader.dll"
]
```

### Elden Mod Loader
If you are using [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117) to load DLL mods, simply place the DLL and its config file in your mods folder.

## Configuration
The JSON config file that comes with the DLL has some options that you can change to your needs:
```jsonc
{
  /*
    Set this to true to enable the console, which will display extra
    information about what the DLL is doing.
  */
  "log": false,

  /*
    This is the port number used for the WebSocket server. Feel free to change
    it to whatever you'd like if you can't use the default for some reason.
  */
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
- `requestID`: A string used to identify the request. The server doesn't use this for anything. It simply includes it in the response to that request so that the client can know what request the response was for.
- `type`: The type of the request, which tells the server what to do. It has two valid values:
  - `0`: Reload FXR. This patches the definitions for the given FXR so that any new instances of it will use the new FXR. When this request type is used, the request needs one additional property:
    - `data`: The binary data of the FXR encoded as a base64 string.
  - `1`: Set resident SFX. This edits the resident SFX param fields for a given weapon based on the properties of the request. When this request type is used, the request needs three additional properties:
    - `weapon`: The numerical ID of the weapon to edit. You can find a list of these here: https://github.com/MaxTheMiracle/Dark-Souls-3-Parts-Files/blob/master/Elden%20Ring
    - `sfx`: The numerical SFX ID to change the `residentSfxId_1` param field to.
    - `dmy`: The numerical dummy poly ID to change the `residentSfx_DmyId_1` param field to.

#### Reload FXR example request payload
```json
{
  "requestID": "example_request_1",
  "type": 0,
  "data": "<base64 goes here>"
}
```

#### Set resident SFX example request payload
```json
{
  "requestID": "example_request_2",
  "type": 1,
  "weapon": 24050000,
  "sfx": 402030,
  "dmy": 206
}
```

### Responses
The server will respond to all requests sent to it with a JSON object containing the request ID and a status message. The status message will always be "success" if the process succeeded. If it failed, the status message will describe why it failed in some way.

#### Success example response
```json
{
  "requestID": "example_request_2",
  "status": "success"
}
```

#### Error example response
```json
{
  "requestID": "example_request_1",
  "status": "Invalid FXR"
}
```
