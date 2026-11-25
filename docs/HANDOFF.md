# BeyGram Installer Foundation handoff

Work is on `feature/mvp0-foundation` in [Draft PR #10](https://github.com/Beykus-Y/BeyGram-Installer/pull/10) for [Issue #1](https://github.com/Beykus-Y/BeyGram-Installer/issues/1). The development base is `DEV`. Its histories join at `bc0320389fe6d540b28a291c757c91c39d3ecffe`. The upstream v2.7.3 commit is `2ba32ea43246bd4db0265eb3120d3c72e39f5b22`. The previous audit is in [MVP0_REPORT.md](MVP0_REPORT.md).

## MVP 0.1 changes

- Workspace rustfmt baseline is in the isolated `737fd1a` commit. `31ca5e7` removes automatic certificate revocation from GUI, CLI and refresh. Apple quota error 7460 directs the operator to the team owner and Apple's developer portal.
- `ebc4f5a` verifies HTTPS/WSS, rejects redirects and constrains Apple provisioning URLs to `gsa.apple.com:443`. Application entry points use local StoreServicesCore. Remote Anisette requires an explicit HTTPS provider configuration. The GUI login screen and CLI identify local Anisette. Provider data disclosure, deterministic fixtures and separately runnable live cases are in [BUILD.md](BUILD.md).
- `62bfe42` protects managed account data, signing keys, cached remote pairing records and provider-scoped ADI state with OS-backed encryption. Valid account, key and pairing records migrate in place. `c5258fc` prevents a stale store from overwriting a newer revision, including after logout. Automatic signing-key and pairing-record transfers to recognized apps were removed. Explicit PKCS#12 export uses a generated password.
- `e296b19` removes wire payloads from application diagnostics, blocks sensitive dependency log targets and switches CLI login to a masked password prompt.
- `96b7973` and its follow-ups isolate IPA extraction in private directories, restrict archive entry types and bound preflight metadata reads. `8d73121` updates only dependencies relevant to the advisory triage. [RustSec disposition](RUSTSEC_MVP01.md) records all 49 baseline matches and 18 remaining lockfile matches. The [licensing plan](LICENSES.md#binary-distribution-release-gate) lists the outstanding release decisions.

## Verification and next action

The [first hardening matrix](https://github.com/Beykus-Y/BeyGram-Installer/actions/runs/45558623948) passed on Linux, Windows and macOS after the Anisette changes. The [credential and logging matrix](https://github.com/Beykus-Y/BeyGram-Installer/actions/runs/45559496918) also passed. Linux focused checks passed for Anisette, migration, authenticated-record rejection and stale-write rejection. The native Linux vault smoke test passed using synthetic data in an isolated GNOME Keyring session.

The [PR checks](https://github.com/Beykus-Y/BeyGram-Installer/pull/10/checks) record the active head and full workspace matrix after ZIP and dependency changes. Query them with `gh pr checks 10 --repo Beykus-Y/BeyGram-Installer`. The explicitly invoked live Anisette tests and native Windows/macOS vault smoke checks need their respective prerequisites. No real Apple ID, physical device, signed release or published binary was used for MVP 0.1. Retain draft status while verification is incomplete. Issue #2 device and IPA work remain outside this milestone.
