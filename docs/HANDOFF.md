# MVP 0 handoff

Development base is `DEV`, with original histories joined at `bc0320389fe6d540b28a291c757c91c39d3ecffe`. Upstream v2.7.3 is `2ba32ea43246bd4db0265eb3120d3c72e39f5b22`. Audit work is published on `feature/mvp0-foundation` in [Draft PR #10](https://github.com/Beykus-Y/BeyGram-Installer/pull/10) for [Issue #1](https://github.com/Beykus-Y/BeyGram-Installer/issues/1).

```sh
git fetch origin
git switch feature/mvp0-foundation
git log -1 --format='%H %s' origin/feature/mvp0-foundation
git merge-base --is-ancestor 2ba32ea43246bd4db0265eb3120d3c72e39f5b22 origin/DEV
git merge-base --is-ancestor 7d44044be60ddc5fa758709d37d81ced8dc7499c origin/DEV
gh pr checks 10 --repo Beykus-Y/BeyGram-Installer
```

The latest code checkpoint is `04f9eb84736e52c6cb5e7c5aa6da762cbc550751`. The remote branch head identifies the latest saved commit, including updates to this file. The verified import, source audit, security and licensing findings and Issue #2 recommendations are in [MVP0_REPORT.md](MVP0_REPORT.md) and its linked documents. The project keeps the upstream signing engine and architecture. Future syncs merge reviewed upstream commits into DEV. Foundation CI checks workspace binaries and tests without publishing artifacts.

Locked metadata resolved. The native CI rerun built both binaries and completed Clippy on Ubuntu, Windows and macOS. The two live Anisette tests failed on all runners. Rustfmt reproduces upstream differences locally and in CI. The Debian container retry built both binaries and the CLI help command ran. Workspace tests compiled and reported two Anisette errors, and local Clippy completed with warnings. See [BUILD.md](BUILD.md) for exact commands, versions and logs. RustSec found advisories. No physical device testing has happened.

For the remaining CI work, reproduce the two Anisette failures with `cargo test --locked -p omnisette --lib --features remote-anisette-v3`, define a deterministic test fixture, and review the baseline rustfmt diff. Keep the PR in draft while verification is incomplete. Issue #2 requires two real IPA artifacts and a separate device/team test without publishing credentials.

## MVP 0.1 checkpoints

- `737fd1a` applies the baseline rustfmt diff in a dedicated commit.
- `31ca5e7` removes the Developer API revocation call and GUI confirmation path. Quota error 7460 requires the team owner to manage certificates in Apple's portal.
- Anisette uses HTTPS with certificate verification and rejects redirects. Apple provisioning URLs must use `gsa.apple.com:443`. The remote transport uses WSS and explicit provider selection. CLI, GUI and refresh use local StoreServicesCore by default. The focused Anisette suite passed locally on Linux. See [BUILD.md](BUILD.md) for the separate live target.

Next P0 work is protecting account tokens, signing keys and pairing records, then verifying diagnostic logging. P3 advisory applicability and licensing work remain open. Keep PR #10 in draft pending the full native matrix and security criteria.

Managed account records, signing keys, cached remote pairing records and provider-scoped remote Anisette state now use local authenticated encryption. The encryption keys live in Windows Credential Manager, macOS Keychain or Linux Secret Service. Valid older account, key and pairing records are replaced in place after validation and vault access. Signing and installation no longer copy a signing key or pairing record into a recognized third-party app automatically. The explicit certificate export uses a generated password and the explicit device pairing transfer remains available.

Developer and authentication wire payloads are excluded from diagnostics. The app logger blocks sensitive dependency log targets even when verbose logging is requested. CLI login uses a masked password prompt. Apple request and Anisette protocol errors no longer expose raw developer responses or WebSocket bodies.
