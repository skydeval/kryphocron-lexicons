# Changelog

All notable changes to kryphocron-lexicons are documented
here. The format follows [Keep a Changelog]; this project
adheres to [Semantic Versioning] with the 0.x caveat that
0.x minor bumps may carry breaking changes per Rust ecosystem
convention.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

## [0.3.4] — unreleased

### Added
- `postPrivate.mediaKinds` (array of strings, known values `image` and `video`, at most two). An encoding host that carries a post's `embed` inside `encodedContent` leaves in the clear the kinds of media the post's own embed carries, so AppViews can filter an author's posts by media without decoding. Optional, derived from the encoded embed, and absent when the post has no media. Link cards, plain quotes and a quoted post's own media do not count; a record-with-media embed counts its own media.

### Note for Rust consumers
- The generated `postPrivate` `Main` struct gains one field. Code that constructs it with a struct literal needs updating; deserialization and field access are unaffected.

## [0.3.3] — 2026-10-19

### Fixed
- `graph.block` and `graph.mute` descriptions, the README and `docs/design/lexicon-set.md` no longer say block and mute records are unfederated or private to their owner. Like any repo record they can reach sync channels such as the firehose, so who blocked or muted whom is observable there.

## [0.3.2] — 2026-10-19

### Added
- `postPrivate.embedBlobs` (array of blobs), `postPrivate.mentionDids` (array of DIDs) and `postPrivate.quote` (`postPublic#strongRef`). An encoding host that carries a post's `facets` and `embed` inside `encodedContent` leaves these in the clear: the embed's blobs so it keeps them, and the mentioned DIDs and quoted record so AppViews can index mentions and quotes without decoding. All three are optional, derived from the encoded content, and absent when there is nothing to derive.

### Fixed
- `postPrivate`'s description, the README and `docs/design/lexicon-set.md` no longer say private posts are never federated. A private post can still reach sync channels such as the firehose; the audience gate is access friction, not encryption.

### Note for Rust consumers
- The generated `postPrivate` `Main` struct gains three fields. Code that constructs it with a struct literal needs updating; deserialization and field access are unaffected.

## [0.3.1] — 2026-10-15

### Added
- `postPublic#embedVideo` (with `#caption`) and `postPublic#embedRecordWithMedia` (a quoted record plus images, video or a link card), both in `postPrivate`'s `embed` union. Shapes mirror `app.bsky.embed.video` and `app.bsky.embed.recordWithMedia`.
- `postPublic#embedGallery` (up to 20 `#galleryImage` items, each with a required `aspectRatio`), in `postPrivate`'s `embed` union and `#embedRecordWithMedia`'s `media` union. Mirrors `app.bsky.embed.gallery`.
- `postPublic#aspectRatio`; `#image` gains optional `aspectRatio`. `#external` gains optional `thumb` (image blob, max 1 MB).
- `postPrivate.langs` (up to three language tags) and `postPrivate.labels` (`postPublic#selfLabels`), mirroring `app.bsky.feed.post`.
- Procedure lexicons for the private-tier write endpoints: `feed.createPostPrivate`, `feed.updatePostPrivate`, `feed.deletePostPrivate`, `actor.participatePrivate`, `policy.manageAudience`, `policy.deleteAudience`. They ship as JSON and load through `lexicons()`; they are not in the registry or lockfile, and no Rust codegen is emitted for them.
- Manifest entries take an optional `kind` (`record`, the default, or `procedure`). The build checks it against the lexicon's `main` def, requires a tier on records and rejects tier fields on procedures.

### Changed
- `postPublic#image` blob `maxSize` raised from 1 MB to 2 MB, matching `app.bsky.embed.images`.
- Codegen output is cleared before each build, so a removed lexicon can no longer leave stale generated files behind.
- `version.json` lexicon-set version moved from `0.1.0` to `0.3.1`. It had not been bumped since 0.1.0; it now matches the crate version. Existing lockfile entries keep their `first_seen_in_lexicon_set_version`.

### Fixed
- README and `SECURITY.md` no longer claim the published crate ships pre-generated types or that `KRYPHOCRON_CODEGEN_HASH` is checked against a committed value. Every build, including downstream ones, needs `proto-blue-codegen` installed. `SECURITY.md` now lists 0.3.x as supported.

### Note for Rust consumers
- The generated `Main` structs and embed enums gain fields and variants. Code that constructs them with struct literals or matches them exhaustively needs updating; deserialization and field access are unaffected.

## [0.3.0] — 2026-06-13

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
