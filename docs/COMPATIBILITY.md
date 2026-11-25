# Platform compatibility

| Platform | Existing upstream paths | BeyGram MVP target |
|---|---|---|
| Windows | USB and network entries from the Apple-compatible usbmux stack, installer/profile services, current-user tray autostart. Upstream packages x86/x64 GNU and ARM64 MSVC builds. | USB onboarding, confirmed Wi-Fi discovery, automatic renewal and IPA updates, reconnect and USB fallback. Test Windows 11 first, then Windows 10 where available. |
| Linux | USB through usbmuxd, GTK tray, XDG autostart and upstream AppImage packaging. Runtime requires compatible desktop and device services. | USB install, manual update and manual renewal. Fedora is an explicit acceptance target. |
| macOS | Native usbmux services, AppKit/tray and LaunchAgent support. Upstream packages a universal bundle. Apple Silicon local Mac installation is a separate existing path. | USB install, manual update and manual renewal for iPhone. |

On Windows the upstream [README](../.github/README.md) gives the Apple support requirements. The Linux/macOS wireless servicing paths are outside BeyGram MVP. Windows discovery recovery, multiple transport handles for one UDID and physical device profile-only renewal remain acceptance tests. The desktop CI matrix compiles and runs available automated tests. It cannot exercise Apple USB drivers, Trust dialogs, signing against a real team, background renewal or data persistence on an iPhone.

See [Build](BUILD.md) for separate developer-machine, CI and hardware results.
