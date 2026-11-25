# MVP 0 handoff

Development base is `DEV`, with original histories joined at `bc0320389fe6d540b28a291c757c91c39d3ecffe`. Upstream v2.7.3 is `2ba32ea43246bd4db0265eb3120d3c72e39f5b22`. Audit work is published on `feature/mvp0-foundation` in [Draft PR #10](https://github.com/Beykus-Y/BeyGram-Installer/pull/10) for [Issue #1](https://github.com/Beykus-Y/BeyGram-Installer/issues/1).

```sh
git fetch origin
git log -1 --format='%H %s' origin/feature/mvp0-foundation
git merge-base --is-ancestor 2ba32ea43246bd4db0265eb3120d3c72e39f5b22 origin/DEV
git merge-base --is-ancestor 7d44044be60ddc5fa758709d37d81ced8dc7499c origin/DEV
gh pr checks 10 --repo Beykus-Y/BeyGram-Installer
```

The remote branch head identifies the latest saved commit, including updates to this file. The verified import, source audit, security and licensing findings and Issue #2 recommendations are in [MVP0_REPORT.md](MVP0_REPORT.md) and its linked documents. The project keeps the upstream signing engine and architecture. Future syncs merge reviewed upstream commits into DEV. Foundation CI checks workspace binaries and tests without publishing artifacts.

Locked metadata resolved. The native CI rerun built both binaries and completed Clippy on Ubuntu, Windows and macOS. The two live Anisette tests failed on all runners. Rustfmt reproduces upstream differences locally and in CI. The full local Linux retry is pending. See [BUILD.md](BUILD.md) for exact commands, versions and logs. RustSec found advisories. No physical device testing has happened.

Next record the local Linux retry result. For the remaining CI work, reproduce the two Anisette failures with `cargo test --locked -p omnisette --lib --features remote-anisette-v3`, define a deterministic test fixture, and review the baseline rustfmt diff. Keep the PR in draft while verification is incomplete. Issue #2 requires two real IPA artifacts and a separate device/team test without publishing credentials.
