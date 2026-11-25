# BeyGram Installer

Cross-platform desktop installer and lifecycle manager for BeyGram iOS, based on [Impactor](https://github.com/claration/Impactor).

**Status: MVP 0 Foundation.** DEV contains Impactor v2.7.3 and the BeyGram roadmap. The Foundation audit and checks are tracked in [Issue #1](https://github.com/Beykus-Y/BeyGram-Installer/issues/1) and [Draft PR #10](https://github.com/Beykus-Y/BeyGram-Installer/pull/10). BeyGram Installer has no published release.

## Product goal

Install BeyGram on iPhone with a free Apple Account, then minimize manual maintenance. Windows is the primary platform for automatic Wi-Fi maintenance. Release and DEV can coexist as independent apps.

### Target platform matrix (MVP, not implemented yet)

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| Initial USB setup and installation | Yes | Yes | Yes |
| Install Release and DEV simultaneously | Yes | Yes | Yes |
| Manual USB update and signing renewal | Yes | Yes | Yes |
| Automatic discovery and signing renewal over Wi-Fi | Yes | No | No |
| Automatic IPA updates over Wi-Fi | Yes | No | No |
| Background maintenance and autostart | Yes | Not in MVP | Not in MVP |

Windows 11 and a modern physical iPhone are the primary acceptance environment. Windows 10, Fedora/Linux and macOS support must be validated before claiming compatibility. Wi-Fi on Linux/macOS is explicitly outside MVP scope.

## User experience

1. Download the installer for your desktop OS.
2. Connect the iPhone via USB and confirm Apple's trust/developer prompts.
3. Sign in with an Apple Account and select Release, DEV, or both.
4. Install verified IPA packages with stable app identities.
5. On Windows, enable optional autostart and verify Wi-Fi connectivity with USB disconnected.
6. The Windows agent checks for new builds and renews signing before expiry when the PC and iPhone are reachable. Linux/macOS users can manually update or renew via USB.

Free Apple Personal Team signing is time-limited, and Apple applies app and App ID quotas. The installer must reuse existing identities, not create new IDs for each build. Apple may require user interaction or reauthentication. No guarantee of renewal while the PC is off.

## Release and DEV

Two independent app identities, data containers, keychain mappings, extension IDs, and update channels. Actual identifiers must be verified against BeyGram iOS artifacts; the proposed logical IDs are `com.beykus.beygram` and `com.beykus.beygram.dev`. Persist the effective mapping for each Apple Team. Count extension App IDs in quota preflight. Never automatically uninstall an existing app, revoke unrelated certificates, or switch channels.

## Development roadmap

| Stage | Issue | Outcome |
|---|---|---|
| MVP 0 | [#1](https://github.com/Beykus-Y/BeyGram-Installer/issues/1) | Import upstream with history; audit baseline |
| MVP 1 | [#2](https://github.com/Beykus-Y/BeyGram-Installer/issues/2) | Validate real IPA signing and fixed dual identities |
| MVP 2 | [#3](https://github.com/Beykus-Y/BeyGram-Installer/issues/3) | USB onboarding and dual-channel installation |
| MVP 3 | [#4](https://github.com/Beykus-Y/BeyGram-Installer/issues/4) | Windows-only Wi-Fi pairing and recovery |
| MVP 4 | [#5](https://github.com/Beykus-Y/BeyGram-Installer/issues/5) | Windows background agent, autostart and renewal |
| MVP 5 | [#6](https://github.com/Beykus-Y/BeyGram-Installer/issues/6) | Verified release feed and automatic updates |
| MVP 6 | [#7](https://github.com/Beykus-Y/BeyGram-Installer/issues/7) | Security, branding, packaging and diagnostics |
| MVP 7 | [#8](https://github.com/Beykus-Y/BeyGram-Installer/issues/8) | End-to-end QA and release gate |
| Post-MVP | [#9](https://github.com/Beykus-Y/BeyGram-Installer/issues/9) | Signing status and local installer integration inside BeyGram iOS |

Start with #1, then #2. Stages #3 and #4 may proceed in parallel after the signing feasibility gate. #5 depends on #4; #6 depends on stable identities and installation, and Windows automation depends on #4/#5. Security checks apply from the start. #8 gates the MVP release; #9 is not required for MVP.

## Engineering and security

- Preserve Impactor history, copyright notices, license texts and upstream sync capability. Audit dependencies before declaring the final license.
- Use a dedicated DEV branch after upstream import; submit implementation PRs to DEV and merge validated releases to main.
- Keep Apple credentials, private keys, session tokens and pairing data local and protected by OS facilities. Audit any third-party authentication services.
- Verify signed release metadata and IPA integrity. Keep the last working package available for recovery.
- Preserve stable identifiers and user data on updates. Fail safely; no silent destructive recovery.
- Prefer profile-only renewal when supported and verified; fall back to safe re-signing if necessary.
- Record real-device tests and known limitations. Planned functionality is not a compatibility claim.

## Upstream

[claration/Impactor](https://github.com/claration/Impactor) is the implementation base. Its history and the original BeyGram planning history meet in the DEV import merge. See [upstream synchronization](docs/UPSTREAM.md) and the [original Impactor README](.github/README.md).

## Foundation documentation

- [Architecture](docs/ARCHITECTURE.md), [signing and renewal](docs/SIGNING.md), and [platform compatibility](docs/COMPATIBILITY.md)
- [Build and verification](docs/BUILD.md), [security](docs/SECURITY.md), and [licensing](docs/LICENSES.md)
- [MVP 0 report](docs/MVP0_REPORT.md) and [handoff](docs/HANDOFF.md)

See [Issues](https://github.com/Beykus-Y/BeyGram-Installer/issues) for Russian-language implementation requirements and acceptance criteria.
