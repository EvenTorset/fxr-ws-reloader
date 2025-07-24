# Changelog

## v3.3.0
### Supported games
- **Armored Core VI** 🔄️🪝
- **Dark Souls III** 🔄️🪝
- **Elden Ring** 🔄️🎛️🪝
- **Elden Ring Nightreign** 🔄️🪝
- **Sekiro** 🔄️🪝

(May not work for all versions of the games. Features: 🔄️ = reload, 🎛️ = params, 🪝 = extract)

### Changes
- Reloading should now work across pretty much every version of the supported games. This fixes the reloader not working for NR 1.01.4 and 1.01.5.
- Added an option to open a console window for the reloader to output useful information to. The console window can be enabled by setting the `console` property in the config file to `true`.
- Subsequent non-params requests should now be slightly faster. The first request may be a bit slower.

## v3.2.0
### Supported games
- **Armored Core VI** 🔄️🪝
- **Dark Souls III** 🔄️🪝
- **Elden Ring** 🔄️🎛️🪝
- **Elden Ring Nightreign** 🔄️🪝
- **Sekiro** 🔄️🪝

(May not work for all versions of the games. Features: 🔄️ = reload, 🎛️ = params, 🪝 = extract)

### Changes
- Added support for Nightreign 1.01.2. Dropped support for earlier versions.
- Added a way to request extraction of multiple FXRs at once.

## v3.1.0
### Supported games
- **Armored Core VI** 🔄️🪝
- **Dark Souls III** 🔄️🪝
- **Elden Ring** 🔄️🎛️🪝
- **Elden Ring Nightreign** 🔄️🪝
- **Sekiro** 🔄️🪝

(May not work for all versions of the games. Features: 🔄️ = reload, 🎛️ = params, 🪝 = extract)

### Changes
- Added partial support (🔄️🪝) for **Dark Souls III** and **Sekiro**.
- Added an injector program that can be run to inject the reloader into any of the supported games while they are running.
  - Don't use it while EAC is active, I didn't test it, I don't know what would happen. Its primary use is to support DS3 and Sekiro, since they have limited support for DLL mods that aren't dinput8 hooks.
  - You may need to tab back into the game after injecting to make the reloader server start up.

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
