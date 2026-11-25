# MVP 0.1 RustSec disposition

Baseline `670aa31`, cargo-audit 0.23.0 and RustSec database `10db6998b5d74ec438bcf0763125bd425a068a8f` reproduce 49 vulnerability matches. IDs below are RustSec ID suffixes. A match is one advisory per locked package version. Informational warnings are separate.

The baseline graph was inspected using `cargo tree --locked --target <target> -e normal -p plumeimpactor -p plumesign` and `cargo tree --locked --target <target> -e normal,build,dev --workspace` for x86_64 Linux GNU, x86_64 Windows MSVC and aarch64 macOS. Inverse trees and call sites distinguish host-side proc macros and library functions from reachable runtime paths. The macOS Intel and Windows GNU packaging targets still require artifact-level verification.

| Package and IDs | Matches | Targets, features and disposition |
|---|---:|---|
| crossbeam-epoch 0.9.18, 2026-0204 | 1 | All three, runtime via Rayon, `alloc,std`. Hazard is pointer formatting with an invalid pointer, with no installer call to the affected format path. Patched in lockfile. |
| crypto-bigint 0.5.5 and 0.7.3, 2026-0387 | 2 | All three, runtime. 0.7 via SRP `alloc,default,getrandom,rand_core` handles Apple authentication secrets and is patched. 0.5 via vendored P-256/elliptic-curve `generic-array,rand_core,zeroize` can handle a user-supplied EC signing key. The normal Apple-identity path generates RSA keys. No interactive remote EC signing oracle was found. Remaining 0.5 risk is local/adjacent timing for an imported EC key and needs a coordinated P-256 upgrade or provider replacement. |
| ctutils 0.4.2, 2026-0385 | 1 | All three, runtime via SRP crypto-bigint. Secret-dependent comparisons are relevant to authentication even though timing exposure depends on the peer and platform. Patched. |
| h2 0.4.13, 2026-0258 | 1 | All three, runtime via Hyper/reqwest `http2`. Unbounded frame handling affects network availability when speaking HTTP/2. Patched. |
| openssl 0.10.78, 2026-0474, 0476, 0477 | 3 | Absent from the three tested runtime/build graphs. `native-tls` uses Schannel on Windows and Security Framework on macOS, and the Linux native-TLS dependency is inactive. The matching APIs are OCSP responder string conversion and AES wrap operations. Reassess if OpenSSL becomes active on a supported target. |
| quick-xml 0.37.5, 2026-0194, 0195 | 2 | Windows GUI runtime only via `notify-rust → tauri-winrt-notification`, default features. Windows notification XML is constructed locally. No path was found from an external XML document to the affected notification parser or `NsReader`. Follow up on a compatible upstream notification dependency update. |
| quick-xml 0.39.2, 2026-0194, 0195 | 2 | Baseline runtime plist decoding on all three, default features. External XML is reachable via IPA metadata and Apple provisioning responses. Plist now uses quick-xml 0.42. The remaining Linux edge is the Wayland scanner proc macro reading build-time protocol descriptions. `NsReader` exhaustion applies only where that reader handles attacker-controlled XML. |
| quinn-proto 0.11.14, 2026-0185, 0338, 0339, 0340, 0342, 0523, 0524; quinn-udp 0.5.14, 2026-0538 | 8 | Absent from all three compiled graphs. Optional reqwest HTTP/3 is disabled. QUIC stream, datagram, ACK, retry, connection-ID and UDP receive paths are unreachable while HTTP/3 remains disabled. |
| rsa 0.9.10, 2023-0071 | 1 | All three, runtime `default,pem,sha2,std,u64_digit`. Main flow generates/parses RSA keys and signs locally via rcgen/ring. The affected private decrypt operation exists in the vendored upstream remote-signing implementation. Neither BeyGram entry point creates that service or routes external ciphertext to it. Keep it disconnected and plan a stable RSA migration before any remote-signing work. |
| rustls 0.23.40, 2026-0285, 0346, 0348, 0349 | 4 | All three, runtime `ring,aws-lc-rs,std,tls12`. TLS handshake and client-session paths are active in network connections. Brotli certificate decompression is disabled. Keep raw TLS logging suppressed and use the fixed version for all four matches. |
| rustls-webpki 0.103.13, 2026-0382 | 1 | All three, runtime `alloc,aws-lc-rs,ring,std`. Certificate validation reachable in Rustls transports. Patched. |
| ruzstd 0.7.3, 2026-0344 | 1 | All three, runtime graph via vendored `object` default compression, `default,hash,std`. Installer uses Mach-O metadata and writing and has no call to compressed-section decoding. Removed the unused `object/compression` feature and dependency. |
| url 2.5.8, 2026-0592 | 1 | All three, runtime `default,std`, with `serde` on Linux. Affected `Url::origin` path concerns nested blob URLs. The Anisette provider is required to be HTTPS before its origin is logged. Patched as defense in depth. |
| webpki-root-certs 1.0.7, 2026-0383 | 1 | Absent from three compiled graphs. Retained as an optional platform-verifier edge for other targets and patched in the lockfile. |
| webpki-roots 1.0.7, 2026-0381 | 1 | All three, runtime via rustls root consumers including Anisette WSS. Certificate validation supply-chain risk. Patched. |
| xmas-elf 0.9.1, 2025-0018 | 1 | All three, runtime in Android loader. Affected SysV hash table bucket/chain accessors have no call sites in pinned loader, which uses its own GNU hash implementation. The separate trust risk of loading local native code remains. |
| zip 2.4.2, 2026-0367, 0368, 0369, 0429, 0545, 0548 | 6 | All three, `deflate` via vendored apple-codesign, notarization inactive and ZIP extraction/AES unreachable there. Unified with the patched shared ZIP version to remove the stale dependency. |
| zip 8.6.0, 2026-0367, 0368, 0369, 0429, 0545, 0546, 0547, 0548 | 8 | All three, runtime default compression + AES via IPA and tweak handling. Symlink escape, permission and claimed-size hazards were relevant to IPA extraction, with Unix permission hazards specific to Linux/macOS. The stream API is unused, `legacy-zip` is disabled, and users cannot supply an AES ZIP password in these flows. Now uses ZIP 9.0.5, private per-extraction staging, regular files/directories only and bounded preflight metadata reads. |
| zstd 0.13.3, 2026-0391 | 1 | All three, runtime `default,arrays,legacy,zdict_builder`. Decompress and baseline ZIP use Zstd. No reusable cross-input raw context caller found. Patched. |
| zstd-safe 7.2.4, 2026-0398, 0403 | 2 | All three, runtime `legacy,std,arrays,zdict_builder`. Seekable API inactive. Streaming legacy decode is relevant to archive input. Updated through the Zstd upgrade. |
| zstd-sys 2.0.16, 2026-0410 | 1 | All three, native runtime `legacy,std,zdict_builder` and build-script C compilation. The vulnerable allocator shim is wasm32-only. Updated as part of Zstd resolution. |

To repeat the audit on the exact dependency snapshot:

```sh
cargo +1.100.0 metadata --locked --format-version 1 >/dev/null
cargo audit --db /path/to/advisory-db --no-fetch
```

Recheck with the latest database and exact distribution targets before releasing binaries. Feature changes enabling HTTP/3, remote signing, or additional archive readers invalidate the corresponding reachability decisions.

The follow-up scan of the MVP 0.1 lockfile on the same database reports **18 vulnerability matches**, down from 49, and 28 informational warnings (15 unmaintained, 8 unsound, 5 yanked). The 18 are the crypto-bigint 0.5, OpenSSL, two remaining quick-xml versions, optional QUIC, RSA and xmas-elf rows above. The identified critical runtime paths in TLS, Apple SRP/plist and IPA extraction now use patched dependencies or exclude the affected feature. EC import, Windows notification XML, build-time Wayland XML and enabling any currently dormant feature require the follow-up described in the table.
