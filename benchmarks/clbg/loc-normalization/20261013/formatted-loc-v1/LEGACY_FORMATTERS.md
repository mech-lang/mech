# Additional existing-cohort native formatter candidates

Primary documentation read on 2026-10-13 establishes these candidate tool
owners. No local image/version/config/output has been qualified, downloaded or
run here. Every actual formatter is pinned separately from the measured source
compiler; neither newest documentation nor an executable name supplies a local
formatter receipt.

| Source language | Official project route | Boundaries before actual formatting |
| --- | --- | --- |
| Mech | Repository `src/cli/commands/format/mod.rs`: `mech format SOURCE --out OUTPUT`, raw-text branch uses `CanonicalDocumentRenderer.format_pretty_text`. | Exact CLI image and original compiled package provenance must be pinned. The command loads stylesheet/shim resources even in text mode; explicit local pinned resource inputs must prevent network/ambient fallback. Copy-only input/output, no application run. |
| C / C++ | [LLVM clang-format documentation](https://clang.llvm.org/docs/ClangFormat.html), with tagged LLVM ownership source listed in FORMATTERS.md. | Explicit style and `--sort-includes=false`; no implicit `.clang-format` search. Whole helper bundle and independently qualified lexical masks remain required. |
| Rust | [Rust toolchain cargo-fmt reference](https://doc.rust-lang.org/cargo/commands/cargo-fmt.html), [rust-lang rustfmt source/interface](https://github.com/rust-lang/rustfmt). | Prefer explicit isolated files rather than Cargo package discovery. Pin formatter edition/style_edition/config and every authored module. Out-of-line module formatting must not reach originals. No rustfmt installation or execution is authorized here. |
| Go | [Go command inventory](https://go.dev/doc/cmd), [official gofmt interface](https://go.dev/cmd/gofmt/). | Plain explicit-file gofmt, without rewrite or simplify flags. Actual distribution identity and output receipts still required. |
| Swift | [Swift.org source tooling](https://www.swift.org/documentation/source-code/), [swiftlang swift-format source/interface](https://github.com/swiftlang/swift-format). | The language project supplies swift-format; Swift 6 toolchains include it. Existing measured Swift 5.10 does not imply an installed or matched formatter. Select a source-compatible exact formatter image/config separately. Third-party similarly named SwiftFormat is not substituted. |
| Dart | [Official Dart format CLI](https://dart.dev/tools/dart-format), [Dart SDK command inventory](https://dart.dev/tools/dart-tool). | Explicit copied files; pin SDK, language version and formatter configuration. The formatter can add/remove trailing commas and move comments; preserve exact actual output and review selected source forms. A copied main does not authorize package-wide traversal or app execution. |
| Mojo | [Official compiler command inventory](https://docs.modular.com/mojo/cli/index.html), [publisher formatter interface](https://mojolang.static.modular.com/docs/cli/format/). | A candidate `mojo format` route exists. The pinned measured distribution's formatter/delegated tooling needs exact ownership and version review before admission; current documentation alone is insufficient. No generic Python/Ruff substitute is admitted by name. |

Other existing languages remain pending/null until an affirmative primary-source
review establishes a whole-source language-project formatter or an explicit
unavailable disposition. A missing PATH entry is not evidence of absence.
Term printers, deparsers, reconstructed compiler output and third-party tools
do not automatically qualify as native original-source formatters. Drivers for
official APIs need independent original-reader/comment/config preservation
review. A pending or unavailable helper makes the total null even where another
file's language has a valid candidate above.
