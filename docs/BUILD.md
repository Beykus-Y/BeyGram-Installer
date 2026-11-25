# Build and check Impactor

Use the pinned Foundation CI toolchain, Rust 1.100.0 with rustfmt and Clippy. CI sets `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` to reduce build storage. The upstream [contribution guide](../.github/CONTRIBUTING.md) describes CMake and C++ requirements. The release workflow is authoritative for packaging.

- On Ubuntu/Debian, install a C/C++ toolchain, CMake, pkg-config and the GTK/tray and graphics development packages. On Ubuntu 24.04, use the list in [Foundation CI](../.github/workflows/foundation.yml). On Debian 13 substitute `libayatana-appindicator3-dev` for `libappindicator3-dev`. On Fedora use the upstream development packages and the distro equivalents for secret storage and tray dependencies.
- On Windows, install Visual Studio 2022 Build Tools with C++ and the Windows SDK. The native Foundation CI runner uses MSVC. Upstream x86/x64 packaging cross-compiles GNU targets from Linux using MinGW and NASM, while upstream ARM64 builds natively with MSVC. `package/windows/installer.nsi` feeds NSIS.
- On macOS, install Xcode or its Command Line Tools. `.cargo/config.toml` contains the Darwin linker flags for both architectures. A native Foundation CI compile leaves universal assembly, codesigning and notarization to the upstream release workflow.

Run these commands independently from the repository root.

```sh
rustup toolchain install 1.100.0 --profile minimal --component rustfmt,clippy
cargo +1.100.0 metadata --locked --format-version 1 --no-deps
cargo +1.100.0 metadata --locked --format-version 1
cargo +1.100.0 build --locked --bins --workspace
cargo +1.100.0 fmt --all -- --check
cargo +1.100.0 clippy --locked --workspace --all-targets
cargo +1.100.0 test --locked --workspace
cargo audit --db /path/to/rustsec-advisory-db --no-fetch
```

For the historical audit, use cargo-audit 0.23.0 and check out RustSec advisory database commit `10db6998b5d74ec438bcf0763125bd425a068a8f` before passing the local DB path. Use current advisories for new release assessments.

Upstream developer entry points are `cargo run --bin plumeimpactor` and `cargo run --bin plumesign -- --help`. Upstream packaging entry points include `make linux PROFILE=release APPIMAGE=1`, `make windows PROFILE=release NSIS=1` and `make macos PROFILE=release`. The Makefile constructs a Cargo flag from PROFILE, so use `release` for those invocations and plain Cargo for debug builds. Follow the release YAML for native macOS slices, universal assembly, bundles and packaging. The imported release and metainfo jobs are scoped to the upstream repository on the Foundation branch.

## Verification record

- Local Debian 13 x86_64 with Rust 1.100.0. Locked workspace and full dependency metadata succeeded. `cargo fmt --all -- --check` exited 1 with diffs in 13 upstream source files. Source formatting is preserved for baseline comparison. A focused rustfmt check passed for the modified MobileGestalt crate files.
- RustSec lockfile scan completed with findings. See [Security](SECURITY.md).
- The original [v2.7.3 upstream release run](https://github.com/claration/impactor/actions/runs/43990731025) succeeded. It covered both Linux architectures, two GNU Windows cross targets, native ARM64 Windows, NSIS, macOS slices and a universal DMG. That is upstream build and packaging evidence.
- The [first Foundation PR check](https://github.com/Beykus-Y/BeyGram-Installer/actions/runs/45551693517) built both binaries on all three runners. Windows and Linux workspace tests and Clippy could not compile `plume_gestalt` because its build script unconditionally linked CoreFoundation. Commit [04f9eb8](https://github.com/Beykus-Y/BeyGram-Installer/commit/04f9eb84736e52c6cb5e7c5aa6da762cbc550751) scopes the framework and the bindings to macOS. Linux `cargo test --locked --offline -p plume_gestalt` completed successfully, with zero unit cases in that crate.
- Local Debian 13 x86_64 container build passed in 7m 01s with the locked workspace and Rust 1.100.0. `plumesign --help` exited successfully. The initial container attempts ran out of disk. The successful retry used `debian:trixie-slim` and installed ca-certificates, gcc, g++, make, CMake, pkg-config, GTK 3 and Ayatana AppIndicator development packages. It set `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=4`.
- Inside that container, `cargo test --locked --workspace --no-fail-fast` compiled and ran the workspace. The two `omnisette` tests failed. The local provider returned `MissingLibraries`, and the remote test returned a plist parse error (`ExpectedEndOfEventStream`). The other workspace unit and doc-test targets reported zero cases. Local `cargo clippy --locked --workspace --all-targets` exited 0 with warnings.
- No Windows or macOS developer machine, iPhone, Apple credentials or live signing acceptance session was used. The `omnisette` workspace tests exercise live Anisette providers. The vendored `apple-codesign` test suite exists outside the listed workspace members and runs separately from `cargo test --workspace`.


### Native Foundation CI after the framework fix

The [post-fix PR run](https://github.com/Beykus-Y/BeyGram-Installer/actions/runs/45552335480) exercised the following checks on commit `04f9eb8`.

| Runner | Workspace metadata | Both workspace binaries | Workspace tests | Clippy |
|---|---|---|---|---|
| Ubuntu 24.04 | Passed | Built | Two Anisette failures | Passed with warnings |
| Windows Server 2022 | Passed | Built | Two Anisette failures | Passed with warnings |
| macOS 15 | Passed | Built | Two Anisette failures | Passed with warnings |

On all three runners, `store_services_core::tests::fetch_anisette_ssc_async` returned `MissingLibraries`. `remote_anisette_v3::tests::fetch_anisette_remote_v3` failed because Rustls could not choose a process-level crypto provider. The GUI and CLI binaries explicitly install one, while the test harness does not. Reproduce these cases with `cargo +1.100.0 test --locked -p omnisette --lib --features remote-anisette-v3`. The format job still reports the upstream differences. None of these checks was disabled.
