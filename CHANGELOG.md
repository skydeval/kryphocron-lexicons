# Changelog

All notable changes to kryphocron-lexicons are documented
here. The format follows [Keep a Changelog]; this project
adheres to [Semantic Versioning] with the 0.x caveat that
0.x minor bumps may carry breaking changes per Rust ecosystem
convention.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

## [0.2.0] — 2026-06-02

### Added
- `lexicons()` accessor returning the lexicon document collection
  for runtime validation. Returns a `&'static
  proto_blue_lexicon::Lexicons` built once per process (via
  `OnceLock`) from the lexicon JSON embedded by the build script,
  suitable for use with `proto_blue_lexicon::validate_record`.
  Complements the metadata-only `KRYPHOCRON_LEXICON_REGISTRY` and the
  codegen `tools::*` typed structs; additive, no existing surface
  changed.

## [0.1.0] — 2026-05-17

Initial publication. Companion crate to [kryphocron].

[kryphocron]: https://crates.io/crates/kryphocron

### Added
- Eight `tools.kryphocron.*` lexicon JSON files covering the
  substrate's v1 wire vocabulary (`feed.postPublic`,
  `feed.postPrivate`, `feed.like`, `feed.repost`,
  `feed.threadgate`, `graph.block`, `graph.mute`,
  `policy.audience`).
- Rust codegen wrappers generated from the lexicon JSON via
  `proto-blue-codegen`, mounted at
  `kryphocron_lexicons::tools::*`.
- `KRYPHOCRON_LEXICON_REGISTRY` — build-time-authoritative
  registry constant consumed by the kryphocron crate as its
  tier classification trust anchor (§5.3 / §5.4).
- `KRYPHOCRON_CODEGEN_HASH` — SHA-256 digest of the
  concatenated codegen output, providing the §5.3 hand-edit-
  rejection check.
- Build-time three-way consistency check between lexicon JSON,
  `kryphocron-manifest.json`, and codegen output. Mismatch in
  any direction is a build failure.
- §5.4 invariant enforcement: every private-tier lexicon
  declares an `audienceList` ref to
  `tools.kryphocron.policy.audience`, or carries
  `audience_exempt: true` with a non-empty `exemption_reason`.
  Enforced in `build.rs` and shadow-checked in
  `tests/lexicon_invariants.rs`.
- `.kryphocron-manifest.lock` — monotonic lockfile pinning
  tier, deprecation, and successor metadata. Stale-lockfile
  errors include a copy-and-commit fix instruction.
- Re-exports of validated ATProto identifier and data types
  from `proto-blue-syntax` (`AtUri`, `Datetime`, `Did`,
  `Handle`, `Nsid`, `RecordKey`, `Tid`) and `proto-blue-lex-
  data` (`BlobRef`, `Cid`, `CidError`).
- `LexiconDoc` re-export from `proto-blue-lexicon` for operator
  tooling.
- Tier vocabulary: `Tier`, `Visibility`, `SemVer`,
  `DeprecationState`, `LexiconRegistryEntry`, `UnknownNsid`,
  `Tier::from_nsid`.

### License
- Rust codegen wrappers (`src/`, `build.rs`, generated output):
  MPL-2.0 (`LICENSE-MPL`).
- Lexicon JSON files (`lexicons/`): CC0-1.0 (`LICENSE-CC0`).
- SPDX expression: `MPL-2.0 AND CC0-1.0`.
