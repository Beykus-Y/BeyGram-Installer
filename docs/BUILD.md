# Build and check Impactor

Use the pinned Foundation CI toolchain, Rust 1.100.0 with rustfmt and Clippy. The upstream [contribution guide](../.github/CONTRIBUTING.md) describes CMake and C++ requirements. The release workflow is authoritative for packaging.

- Ubuntu/Debian: install a C/C++ toolchain, CMake, pkg-config and the GTK/tray and graphics development packages. On Ubuntu 24.04, use the list in [Foundation CI](../.github/workflows/foundation.yml). On Debian 13 substitute `libayatana-appindicator3-dev` for `libappindicator3-dev`. On Fedora use the upstream development packages and the distro equivalents for secret storage and tray dependencies.
- Windows: install Visual Studio 2022 Build Tools with C++ and the Windows SDK. The native Foundation CI runner uses MSVC. Upstream x86/x64 packaging cross-compiles GNU targets from Linux using MinGW and NASM, while upstream ARM64 builds natively with MSVC. `package/windows/installer.nsi` feeds NSIS.
- macOS: install Xcode or its Command Line Tools. `.cargo/config.toml` contains the Darwin linker flags for both architectures. A native Foundation CI compile leaves universal assembly, codesigning and notarization to the upstream release workflow.

Run these independently from the repository root:

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

- Local Debian 13 x86_64 with Rust 1.100.0. Locked workspace and full dependency metadata succeeded. `cargo fmt --all -- --check` exited 1 with diffs in 14 upstream source files. Source formatting is preserved for baseline comparison.
- RustSec lockfile scan completed with findings. See [Security](SECURITY.md).
- The original [v2.7.3 upstream release run](https://github.com/claration/impactor/actions/runs/43990731025) succeeded. It covered both Linux architectures, two GNU Windows cross targets, native ARM64 Windows, NSIS, macOS slices and a universal DMG. That is upstream build and packaging evidence.
- The [first Foundation PR check](https://github.com/Beykus-Y/BeyGram-Installer/actions/runs/45551693517) reproduced the format failure. Native builds, tests and Clippy were still running at this checkpoint.
- The local native host lacks GTK development libraries and system package installation privileges. A Debian 13 Linux container build is in progress. The full Rust container image failed to start for lack of disk space and was removed. The smaller Debian image uses the locally installed Rust toolchain.
- No Windows or macOS developer machine, iPhone, Apple credentials or live signing acceptance session was used. The `omnisette` workspace tests exercise live Anisette providers. The vendored `apple-codesign` test suite exists outside the listed workspace members and runs separately from `cargo test --workspace`.
