# kryphocron-lexicons: design overview

This crate holds the `tools.kryphocron.*` wire vocabulary used by
[kryphocron], plus the build-time machinery that keeps that
vocabulary, its tier classification and the generated Rust types in
agreement.

[kryphocron]: https://crates.io/crates/kryphocron

These documents describe the design as built. They cover:

| Document | Covers |
| --- | --- |
| [overview.md](overview.md) | What the crate is, why it is separate, what it ships. |
| [build-pipeline.md](build-pipeline.md) | Codegen, the consistency checks, and the generated artifacts. |
| [manifest-and-lockfile.md](manifest-and-lockfile.md) | Tier classification, immutability, and the private-tier audience rule. |
| [versioning-and-deprecation.md](versioning-and-deprecation.md) | Version axes, compatibility rules, and deprecate-and-replace. |
| [lexicon-set.md](lexicon-set.md) | Each lexicon: its tier, its fields, and what it means. |

## Why a separate crate

The lexicons are the substrate's wire contract, not a Rust detail.
Keeping them in their own repository and crate does three things:

- **Other tooling can use them directly.** The JSON under `lexicons/`
  is standard ATProto Lexicon and is licensed CC0-1.0. A TypeScript
  or Go client can generate types from it without touching any Rust.
- **Changes are deliberate.** A lexicon change is a release of this
  crate, followed by a dependency bump in kryphocron. It cannot ride
  along with an unrelated substrate change.
- **The vocabulary has its own version.** The lexicon set evolves on
  its own schedule (see
  [versioning-and-deprecation.md](versioning-and-deprecation.md)).

## Repository layout

```
kryphocron-lexicons/
├── lexicons/tools/kryphocron/   lexicon JSON, mirroring the NSID hierarchy
│   ├── feed/                    posts, likes, reposts, threadgates, post procedures
│   ├── graph/                   blocks, mutes
│   ├── policy/                  audience lists, audience procedures
│   └── actor/                   participation procedure
├── kryphocron-manifest.json     sidecar: tier and kind per NSID
├── .kryphocron-manifest.lock    committed state of the immutable manifest fields
├── version.json                 lexicon-set version
├── build.rs                     codegen and validation pipeline
├── src/lib.rs, src/tier.rs      crate surface and tier vocabulary
└── tests/                       invariant and validation tests
```

## What the crate ships

- **Lexicon JSON.** Eight record lexicons and six procedure
  lexicons. See [lexicon-set.md](lexicon-set.md).
- **Generated Rust types** for the record lexicons, produced by
  `proto-blue-codegen` at build time and mounted at
  `kryphocron_lexicons::tools::*`.
- **`KRYPHOCRON_LEXICON_REGISTRY`.** This compiled-in constant gives
  every record NSID's tier and deprecation state. kryphocron uses it
  as its runtime trust anchor.
- **`Tier::from_nsid`.** A lookup generated from the registry. It
  returns `UnknownNsid::NotRegistered` for any NSID outside the
  registry, which makes the namespace closed.
- **`lexicons()`.** Every shipped lexicon document, parsed once into
  a `proto_blue_lexicon::Lexicons` collection. It is meant for use
  with `proto_blue_lexicon::validate_record`.
- **`KRYPHOCRON_CODEGEN_HASH`.** A SHA-256 digest of the generated
  code.
- **The tier vocabulary:** `Tier`, `Visibility`, `UnknownNsid`,
  `SemVer`, `DeprecationState` and `LexiconRegistryEntry`.
- **Re-exports** of the ATProto identifier and data types
  (`Did`, `Nsid`, `AtUri`, `Cid`, `BlobRef`, and others) from
  `proto-blue-syntax` and `proto-blue-lex-data`.

## Why the tier vocabulary lives here

`Tier::from_nsid` is generated from the registry by this crate's build
script. Rust's orphan rules require an inherent `impl Tier` to sit in
the crate that defines `Tier`, so `Tier` is defined here. The other
two placements were ruled out:

- **Keeping `Tier` in kryphocron** would mean this crate depends on
  kryphocron, while kryphocron depends on this crate for the registry.
  That is a cycle.
- **A third crate** just for a two-variant enum adds a published
  crate and buys nothing.

kryphocron re-exports these types at its own crate root, so its users
see a single surface. The type-level machinery built on top of `Tier`
stays in kryphocron, because it belongs to the substrate's threat
model rather than to the lexicon set. That machinery is the tier
witnesses, `Tiered<T, Ti>`, `HasNsid` and `MixedTier`.

## How consumers use it

- **kryphocron** reads `KRYPHOCRON_LEXICON_REGISTRY` from its own
  build script. For each record NSID it emits a sealed
  `impl HasNsid for <generated Main type>` carrying the tier witness.
  At compile time it asserts that the number of impls equals the
  registry length. A registry NSID with no generated type fails to
  compile.
- **Hosts** validate incoming records with `lexicons()` and
  `validate_record`, and use `Tier::from_nsid` to classify NSIDs.
- **Non-Rust clients** generate their own types from the JSON. The
  procedure lexicons exist mainly for them: they describe the host
  endpoints for private-tier writes.

## What this crate does not do

- It does not enforce tiers at runtime. Tier enforcement, the
  deprecation write gate and the lexicon-set version check at sync
  handshakes are all kryphocron's.
- It checks the meaning of a lexicon in only one way: a private-tier
  record must declare an audience reference. Beyond that, whether a
  schema is right is a matter for human review. Treat lexicon changes
  as security-relevant.
- It defines no moderation records (cases, labels, appeals). Those
  belong to whichever component owns moderation.
- It provides no migration tooling for deprecated lexicons. Migration
  is up to operators.
