# License review

The root [LICENSE](../LICENSE) is MIT. The imported tree preserves original authorship, copyright and license files. There is no tracked NOTICE file at the baseline. The root MIT license does not replace component-level licenses.

| Component | Evidence and obligations |
|---|---|
| Impactor GUI, CLI and other first-party workspace members | Workspace MIT metadata. Preserve copyright and permission terms in copies or substantial portions, including binary distributions. |
| `plume_core`, patched `apple-codesign`, Apple packaging/signing crates and X.509/CMS helpers | MPL-2.0 applies at file level. Retain notices and make the corresponding covered source available to recipients when distributing executables. Include modifications to covered files and a clear route to the exact source. Other files in a larger work can retain compatible independent terms. The vendored permission patch is recorded under `patches/`. |
| `omnisette`, `decompress` | Workspace manifests resolve to MIT. Their supplied texts are MPL-2.0 and Apache-2.0 respectively. Preserve both texts and track provenance of inherited and modified files before finalizing distribution. Honor MPL source availability and Apache license, notices and applicable changed-file marking for inherited material. |
| `idevice`, Iced, Goblin, `auto-launcher` | MIT per resolved metadata. Preserve notices and include pinned git revision for `idevice`. |
| reqwest, Tokio, tray-icon, and the Rust TLS stack | reqwest and tray-icon are MIT OR Apache-2.0. Tokio is MIT. Rustls offers Apache-2.0, ISC or MIT. `ring` declares Apache-2.0 AND ISC. AWS-LC components have compound notices. Preserve the relevant license selection and bundled notices per target. |
| Hack Regular and embedded assets | Preserve `apps/plumeimpactor/src/appearance/HACK_LICENSE`. Verify provenance and distribution terms for the custom icon font and packaged artwork before shipping. |
| Embedded ElleKit | `plume_utils/src/tweak.rs` includes `ellekit.deb` (package `ell.kit`, version 1.0). The archive lists no copyright or license file. Determine exact corresponding source, license and redistribution terms before any binary release. Do the same for Apple binaries used by local Anisette and for any Apple runtime required on Windows. |

## Locked dependency graph

`cargo metadata --locked --format-version 1` resolved 938 packages across targets and build/test dependencies on Rust 1.100.0. The manifest review includes permissive MIT, Apache, BSD, ISC, Unicode and Zlib notices, MPL file-level terms, root certificate data under CDLA, and the following open questions:

- `remove-async-await 1.0.1` declares AGPL-3.0 and enters the build as an Omnisette procedural macro. Verify generated-code obligations and whether any macro implementation is incorporated in shipped artifacts. A build-time dependency alone does not establish the license of the resulting installer.
- `android-loader 0.2.0` and its `sysv64 0.1.0` macro declare no license in the locked manifests. Their checked-out source at `dfa86501` has no root license text. Resolve permission before distribution.
- Where `self_cell` offers Apache-2.0 OR GPL-2.0-only or `r-efi` offers a permissive alternative to LGPL, review the chosen license per artifact. AppImage shared libraries and bundled compression/native components require inventory outside Cargo metadata.

Reproduce with `cargo metadata --locked --format-version 1` and `cargo tree --locked --target all -i <crate>`. Before packaging, generate a per-artifact SBOM and notice bundle covering Cargo, native libraries and assets. Confirm source availability and unresolved permissions. This source review does not clear binary redistribution.
