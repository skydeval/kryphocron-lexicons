# kryphocron-lexicons

ATProto lexicon JSON files and thin Rust codegen wrappers for
the `tools.kryphocron.*` namespace — the wire vocabulary used
by [kryphocron], a privacy-first ATProto substrate.

[![License: MPL-2.0 AND CC0-1.0](https://img.shields.io/badge/license-MPL--2.0%20AND%20CC0--1.0-brightgreen.svg)](#license)
[![Crates.io](https://img.shields.io/crates/v/kryphocron-lexicons)](https://crates.io/crates/kryphocron-lexicons)

[kryphocron]: https://crates.io/crates/kryphocron

## What this crate ships

- **Lexicon JSON files** under `lexicons/tools/kryphocron/`
  — the canonical source of truth for the substrate's wire
  format. Public domain (CC0-1.0).
- **Rust codegen wrappers** — type-safe Rust bindings generated
  from the lexicon JSON at build time and mounted at
  `kryphocron_lexicons::tools::*`. Used by the kryphocron crate
  and downstream consumers. MPL-2.0.
- **`KRYPHOCRON_LEXICON_REGISTRY`** — a compiled-in registry
  constant the kryphocron crate consumes as its build-time
  trust anchor for tier classification.
- **`KRYPHOCRON_CODEGEN_HASH`** — a SHA-256 digest of the
  generated code, so two builds can be compared. The generated
  code is never committed and is rebuilt from the lexicons on
  every build, so a hand edit cannot survive a rebuild.
- **Tier vocabulary** — `Tier`, `Visibility`, `SemVer`,
  `DeprecationState`, `LexiconRegistryEntry`, `UnknownNsid`,
  and `Tier::from_nsid`.

## Namespace

All lexicons live under `tools.kryphocron.*`. The crate ships eight
record lexicons:

| NSID | Tier | Notes |
| --- | --- | --- |
| `tools.kryphocron.feed.like` | Public | Field-compatible with `app.bsky.feed.like`. |
| `tools.kryphocron.feed.postPublic` | Public | Field-compatible with `app.bsky.feed.post`. |
| `tools.kryphocron.feed.postPrivate` | Private | Audience-gated via required `audienceList` reference; access friction, not encryption. |
| `tools.kryphocron.feed.repost` | Public | Field-compatible with `app.bsky.feed.repost`. |
| `tools.kryphocron.feed.threadgate` | Public | Thread-level interaction policy. |
| `tools.kryphocron.graph.block` | Private | Consumed by `BlockOracle` outside the normal capability flow. Reaches the firehose like any repo record. |
| `tools.kryphocron.graph.mute` | Private | One-directional; consumed by `MuteOracle`. Reaches the firehose like any repo record. |
| `tools.kryphocron.policy.audience` | Private | The audience-list mechanism itself. |

The Public / Private distinction is enforced structurally at
the type level by the kryphocron crate. This crate is the
build-time-authoritative source for which NSID is in which tier.

It also ships six procedure lexicons, describing the XRPC
endpoints a host exposes for writing private-tier records:

| NSID | Does |
| --- | --- |
| `tools.kryphocron.feed.createPostPrivate` | Create a private post under one of the writer's own audiences. |
| `tools.kryphocron.feed.updatePostPrivate` | Change a private post's `audienceList`. |
| `tools.kryphocron.feed.deletePostPrivate` | Delete a private post. |
| `tools.kryphocron.actor.participatePrivate` | Reply under another account's audience. |
| `tools.kryphocron.policy.manageAudience` | Create or update an audience record. |
| `tools.kryphocron.policy.deleteAudience` | Delete an audience record. |

Procedures carry no tier: they are not in the registry or the
lockfile, and the manifest marks them `"kind": "procedure"`. They
ship as JSON and load through `lexicons()`; no Rust codegen is
emitted for them, since it would be an XRPC client.

## What's new in 0.3.1

Media for private posts. New defs in `feed.postPublic`, referenced
from `postPrivate`'s `embed` union: `#embedGallery` (up to 20
images), `#embedVideo` (with `#caption`) and
`#embedRecordWithMedia` (a quote with images, a gallery, video or
a link card). `#image` gains `aspectRatio` and a 2 MB
limit (was 1 MB); `#external` gains `thumb`. `postPrivate` gains
`langs` and `labels` (`#selfLabels`). All shapes mirror their
`app.bsky.*` counterparts. Plus the six procedure lexicons above.

## What's new in 0.3

`tools.kryphocron.feed.postPrivate` gains optional fields:

- `publicCompanion` — an AT-URI pairing a private post with a
  public-tier companion record.
- `encodedContent` (with `encodedContentCodec` and
  `encodedContentGeneration`) — inline codec output for at-rest
  content transforms, in place of `text`. Exactly one of `text`
  or `encodedContent` is present per record.

`tools.kryphocron.policy.audience` gains an optional `mode` field
(`list`, `everyone`, `followers`, `following`, `nobody`);
`members` and `name` are now optional.

**Breaking:** `postPrivate.audienceList` is now an `at-uri`
string (it was a record-def ref that codegenned to an embedded
object). Consumers parsing 0.2 records as an embedded object must
parse 0.3 records as an `at://…` string.

## Relationship to kryphocron

The kryphocron crate depends on this crate for its wire
vocabulary types and registry constant. Operators consuming
kryphocron get these types transitively; direct dependence on
kryphocron-lexicons is rarely necessary.

If you're building an ATProto service that needs to consume
`tools.kryphocron.*` records without pulling in the full
kryphocron substrate, depending on this crate directly works
— the codegen wrappers are usable independently.

## Build prerequisites

Building kryphocron-lexicons requires `proto-blue-codegen`
installed as a binary in the build environment. The codegen
tool generates Rust types from the lexicon JSON files at
build time.

This applies to **every build that includes this crate**,
including crates that depend on it directly or through
kryphocron. The generated types are not shipped in the
published crate; the build script regenerates them each time,
and fails with the install command if the binary is missing.

To install:

```bash
cargo install proto-blue-codegen --version '~0.3.1'
```

**Why a binary, not a library dependency?** `proto-blue-codegen`
0.3.x ships only as a CLI binary, with no library API to call
from `build.rs`, so the build script runs it as a subprocess.
If `proto-blue-codegen` later exposes a library API, the build
script can call it in-process instead, and the installed binary
will no longer be needed. That change does not touch the wire
format.

### CI integration

In any CI that builds this crate, or a crate depending on it,
install the codegen binary before `cargo build`:

```yaml
- name: Install proto-blue-codegen
  run: cargo install proto-blue-codegen --version '~0.3.1'
- name: Build
  run: cargo build
```

## What this crate ships in detail

- Eight `tools.kryphocron.*` record lexicon JSON files and six
  procedure lexicon JSON files (the tables above).
- Codegen wrappers generated by `proto-blue-codegen` and mounted
  at `kryphocron_lexicons::tools::*`.
- `KRYPHOCRON_LEXICON_REGISTRY` — the build-time-authoritative
  registry constant consumed by the kryphocron crate.
- `KRYPHOCRON_CODEGEN_HASH` — SHA-256 digest of the generated
  code. It is published for comparison, not checked against a
  committed value.
- Build-time three-way consistency check between lexicon JSON,
  the manifest, and the codegen output. Mismatch in any
  direction is a build failure. Each manifest entry's `kind`
  (`record`, the default, or `procedure`) must match its
  lexicon's `main` def; records must carry a tier and procedures
  must not.
- Private-tier structural enforcement: every private-tier lexicon
  either declares an `audienceList` reference (an `at-uri` string
  pointing at a `tools.kryphocron.policy.audience` record) or
  carries `audience_exempt: true` with a non-empty
  `exemption_reason`. Enforced in `build.rs` and shadow-checked in
  `tests/lexicon_invariants.rs`.
- `.kryphocron-manifest.lock` — monotonic lockfile pinning tier,
  deprecation, and successor metadata across builds.
- Tier vocabulary: `Tier`, `Visibility`, `SemVer`,
  `DeprecationState`, `LexiconRegistryEntry`, `UnknownNsid`,
  `Tier::from_nsid`.

### Known limitations

- The crate is build-script-heavy by design. The runtime surface
  is small: a registry constant, the tier vocabulary, and the
  generated `tools::*` types. The load-bearing work is the
  build-time three-way consistency check; the in-tree integration
  tests pin its outputs as invariants rather than driving the
  failure paths (deliberately-broken manifests are unsuited to
  in-tree tests).
- Codegen output is a function of `proto-blue-codegen`'s output
  shape. A `proto-blue-codegen` minor bump that changes the
  emitted types is a breaking change for downstream consumers
  and will be released as a `kryphocron-lexicons` minor bump.
- The primary library-API integration path for
  `proto-blue-codegen` is blocked on upstream exposing a
  `lib.rs`; until then, the build uses the subprocess fallback
  (see [Build prerequisites](#build-prerequisites)).

## License

Dual-licensed under `MPL-2.0 AND CC0-1.0`:

- **Rust codegen wrappers** (`src/`, `build.rs`, generated
  output): MPL-2.0. See [`LICENSE-MPL`](LICENSE-MPL).
- **Lexicon JSON files** (`lexicons/`): CC0-1.0 public domain
  dedication. See [`LICENSE-CC0`](LICENSE-CC0).

The CC0 dedication on the lexicon JSON is intentional: the
wire vocabulary's value is universal, and unencumbered lexicon
files benefit any ATProto service consuming
`tools.kryphocron.*` records.

## Feedback

Issues and discussion live on the [kryphocron] repository's
issue tracker — bugs, design questions, and integration
feedback against either crate are welcome there. Security
reports should follow [kryphocron's SECURITY policy] (the
disclosure channel covers both crates).

[kryphocron's SECURITY policy]: https://github.com/skydeval/kryphocron/blob/main/SECURITY.md

## Project shape

kryphocron-lexicons is the companion crate to
[kryphocron][kryphocron-gh], a privacy-first ATProto substrate
by @skydeval.

[kryphocron-gh]: https://github.com/skydeval/kryphocron
