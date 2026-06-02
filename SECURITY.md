# Security Policy

`kryphocron-lexicons` is the companion crate to [kryphocron].
Security reports against either crate should be filed through
the kryphocron private disclosure channel.

[kryphocron]: https://crates.io/crates/kryphocron

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.2.x   | ✅        |
| < 0.2   | ❌        |

## Reporting

See [kryphocron's SECURITY.md] for the disclosure channel and
the full policy. Reports affecting kryphocron-lexicons
specifically — codegen output integrity, manifest tampering,
build-time consistency check bypass, lexicon JSON
canonicalization — are in scope and welcome.

[kryphocron's SECURITY.md]: https://github.com/skydeval/kryphocron/blob/main/SECURITY.md

## In-scope for this crate specifically

- **Three-way consistency check bypass.** Any path that
  produces a successful build with mismatched lexicon JSON,
  `kryphocron-manifest.json`, and codegen output. The §5.3
  check is supposed to fail loudly on drift in any direction.
- **`KRYPHOCRON_CODEGEN_HASH` hand-edit-rejection bypass.** Any
  path that produces a successful build with edited codegen
  output where the committed hash is not regenerated. The §5.3
  hash check is supposed to make hand-edits a structural
  mismatch.
- **§5.4 audience-invariant bypass.** Any path that ships a
  private-tier lexicon without `audienceList` enforcement or
  without `audience_exempt: true` plus a non-empty
  `exemption_reason`.
- **Manifest lockfile tampering.** Any path that produces a
  successful build with a stale or maliciously-edited
  `.kryphocron-manifest.lock` (tier mutation, deprecation
  reversal, or successor reassignment slipping past the
  monotonicity checks in `build.rs`).
- **Registry trust-anchor drift.** Any path through which the
  `KRYPHOCRON_LEXICON_REGISTRY` constant exposed at runtime
  fails to match the committed manifest at build time.

## Out of scope

- Issues in `proto-blue-codegen`, `proto-blue-lexicon`,
  `proto-blue-syntax`, or `proto-blue-lex-data` — report those
  to the upstream project.
- Issues in operator-supplied lexicon files placed alongside
  kryphocron-lexicons's own lexicons. The crate's registry is
  closed to the `tools.kryphocron.*` namespace; operator-side
  lexicon validation lives in their own build pipelines.
- The `cargo install proto-blue-codegen` step itself — the
  build-environment trust model is the operator's
  responsibility.
