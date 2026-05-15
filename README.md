# kryphocron-lexicons

ATProto lexicon JSON files and thin Rust codegen wrappers for
the `tools.kryphocron.*` namespace — the wire vocabulary used
by [kryphocron], a privacy-first ATProto substrate.

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
- **Tier vocabulary** — `Tier`, `Visibility`, `SemVer`,
  `DeprecationState`, `LexiconRegistryEntry`, `UnknownNsid`,
  and `Tier::from_nsid`.

## Namespace

All lexicons live under `tools.kryphocron.*`. v0.1 ships eight:

| NSID | Tier | Notes |
| --- | --- | --- |
| `tools.kryphocron.feed.like` | Public | Field-compatible with `app.bsky.feed.like`. |
| `tools.kryphocron.feed.postPublic` | Public | Field-compatible with `app.bsky.feed.post`. |
| `tools.kryphocron.feed.postPrivate` | Private | Audience-gated via required `audienceList` reference; never federated. |
| `tools.kryphocron.feed.repost` | Public | Field-compatible with `app.bsky.feed.repost`. |
| `tools.kryphocron.feed.threadgate` | Public | Thread-level interaction policy. |
| `tools.kryphocron.graph.block` | Private | Existence is private; consumed by `BlockOracle` outside the normal capability flow. |
| `tools.kryphocron.graph.mute` | Private | One-directional, viewer-visible-only. |
| `tools.kryphocron.policy.audience` | Private | The audience-list mechanism itself. |

The Public / Private distinction is enforced structurally at
the type level by the kryphocron crate. This crate is the
build-time-authoritative source for which NSID is in which tier.

## Relationship to kryphocron

The kryphocron crate depends on this crate for its wire
vocabulary types and registry constant. Operators consuming
kryphocron get these types transitively; direct dependence on
kryphocron-lexicons is rarely necessary.

If you're building an ATProto service that needs to consume
`tools.kryphocron.*` records without pulling in the full
kryphocron substrate, depending on this crate directly works
— the codegen wrappers are usable independently.

## Build requirement

`proto-blue-codegen` 0.3.x ships as a binary-only crate, so
`build.rs` invokes it as a subprocess. Operators must install
the binary before building:

```bash
cargo install proto-blue-codegen --version '~0.3.1'
```

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

## Project shape

kryphocron-lexicons is the companion crate to
[kryphocron][kryphocron-gh], a privacy-first ATProto substrate
by @skydeval.

[kryphocron-gh]: https://github.com/skydeval/kryphocron
