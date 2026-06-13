# Changelog

All notable changes to kryphocron-lexicons are documented
here. The format follows [Keep a Changelog]; this project
adheres to [Semantic Versioning] with the 0.x caveat that
0.x minor bumps may carry breaking changes per Rust ecosystem
convention.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

## [0.3.0] — UNRELEASED

### Added
- `postPrivate.publicCompanion` (optional AT-URI) — points at a paired public-tier record (the public side of a dual-faced post). Records without it remain valid standalone private posts.
- `policy.audience.mode` (optional) — five visibility modes: `list`, `everyone`, `followers`, `following`, `nobody`. Absence reads as `list`. Encoded as `knownValues` (open at the lexicon layer); the value set is closed substrate-side in `validate_record`.
- `postPrivate.encodedContent` (optional `bytes`, max 1MB) — carries codec output when an at-rest content codec is installed. Inline rather than blob-referenced so the codec output lands in the DAG-CBOR record itself.
- `postPrivate.text` XOR `postPrivate.encodedContent` — exactly one must be present per record (substrate-enforced).
- `postPrivate.encodedContentCodec` (optional `string`, max 128) — operator-namespaced codec identifier (e.g. `laquna/0.2`). Required at the application layer when `encodedContent` is present.
- `postPrivate.encodedContentGeneration` (optional `string`, max 128) — per-record rotation generation mark. Host rewrite-on-rotate jobs select records by lexicographic comparison on this field, so hosts must pick a lex-sortable encoding.

### Changed
- `postPrivate.text` relaxed from required to optional (`required` is now `["createdAt", "audienceList"]`). Each record carries exactly one of `text` or `encodedContent`. The `text` constraints (`maxGraphemes: 300`, `maxLength: 3000`) are unchanged.
- `policy.audience.members` relaxed to optional. The conditional-required rule (required when `mode == "list"`) is enforced substrate-side in `validate_record`.
- `policy.audience.name` relaxed to optional (was required). The `maxGraphemes: 64` + `maxLength: 640` constraints are unchanged.

### Fixed
- **`postPrivate.audienceList` shape corrected** from a record-def ref (which codegenned to an embedded `policy.audience` object) to a plain AT-URI string (`{type: "string", format: "at-uri"}`). The audience reference is by-reference — resolved at read time, with membership changes applying retroactively — and the 0.2.0 encoding had drifted to an embedded shape consumers could not validate against on-disk records. **Migration:** downstream consumers parsing 0.2.0 records as `audienceList: { ... }` (embedded object) must parse 0.3.0 records as `audienceList: "at://..."` (string). The build script's structural validator and `tests/lexicon_invariants.rs` were updated in lockstep.

## [0.2.0] — 2026-06-02

### Added
- `lexicons()` accessor — returns a `&'static proto_blue_lexicon::Lexicons` (built once via `OnceLock` from the embedded lexicon JSON) for runtime use with `proto_blue_lexicon::validate_record`. Additive; complements `KRYPHOCRON_LEXICON_REGISTRY` and the codegen `tools::*` types.

## [0.1.0] — 2026-05-17

Initial publication. Companion crate to [kryphocron].

[kryphocron]: https://crates.io/crates/kryphocron

### Added
- Eight `tools.kryphocron.*` lexicon JSON files covering the v1 wire vocabulary (`feed.postPublic`, `feed.postPrivate`, `feed.like`, `feed.repost`, `feed.threadgate`, `graph.block`, `graph.mute`, `policy.audience`).
- Rust codegen wrappers generated from the lexicon JSON via `proto-blue-codegen`, mounted at `kryphocron_lexicons::tools::*`.
- `KRYPHOCRON_LEXICON_REGISTRY` — build-time-authoritative tier-classification registry consumed by the kryphocron crate.
- `KRYPHOCRON_CODEGEN_HASH` — SHA-256 digest of the codegen output, providing a hand-edit rejection check.
- Build-time three-way consistency check between lexicon JSON, `kryphocron-manifest.json`, and codegen output; any mismatch fails the build.
- Private-tier structural enforcement: every private-tier lexicon declares an `audienceList` reference to `tools.kryphocron.policy.audience`, or carries `audience_exempt: true` with a non-empty `exemption_reason`. Enforced in `build.rs`, shadow-checked in `tests/lexicon_invariants.rs`.
- `.kryphocron-manifest.lock` — monotonic lockfile pinning tier, deprecation, and successor metadata. Stale-lockfile errors include a copy-and-commit fix instruction.
- Re-exported ATProto identifier/data types: `AtUri`, `Datetime`, `Did`, `Handle`, `Nsid`, `RecordKey`, `Tid` (from `proto-blue-syntax`) and `BlobRef`, `Cid`, `CidError` (from `proto-blue-lex-data`).
- `LexiconDoc` re-export from `proto-blue-lexicon` for operator tooling.
- Tier vocabulary: `Tier`, `Visibility`, `SemVer`, `DeprecationState`, `LexiconRegistryEntry`, `UnknownNsid`, `Tier::from_nsid`.

### License
- Rust code (`src/`, `build.rs`, generated output): MPL-2.0 (`LICENSE-MPL`).
- Lexicon JSON (`lexicons/`): CC0-1.0 (`LICENSE-CC0`).
- SPDX expression: `MPL-2.0 AND CC0-1.0`.
