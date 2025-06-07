# Changelog

## v3.0.0
### Supported games
- **Armored Core VI** 🔄️🪝
- **Elden Ring** 🔄️🎛️🪝
- **Elden Ring Nightreign** 🔄️🪝

(May not work for all versions of the games. Features: 🔄️ = reload, 🎛️ = params, 🪝 = extract)

### Changes
- The whole project has been rewritten from scratch in Rust.
- The server now sends a message to clients when they connect that contains useful information about the reloader and the game it is running in, such as the name of the game, the DLL version, and what reloader features are supported for the game.
  - The `features` property in this message is what tells you what features are supported. It is an object where the values of the properties are all boolean, and the keys are:
    - `reload`: (🔄️) Allows FXRs to be reloaded.
    - `params`: (🎛️) Allows params to be read or modified.
    - `extract`: (🪝) Allows FXRs to be listed or extracted.
- Added partial support (🔄️🪝) for **Armored Core VI** and **Elden Ring Nightreign**.
- Added the ability for clients to request a list of loaded FXR IDs or the contents of a loaded FXR file from the game's memory.
- Massively improved the performance of reloading or extracting multiple times. After the initial reload/extract request, subsequent requests will now take much less time to complete.
