//! # kryphocron-lexicons — companion lexicon crate
//!
//! Phase 2 deliverable per §5 of the kryphocron design doc.
//! Ships the eight v1 ATProto lexicons under `tools.kryphocron.*`,
//! the substrate-specific sidecar manifest, the build-time three-
//! way registry consistency check, and the
//! [`KRYPHOCRON_LEXICON_REGISTRY`] constant the kryphocron crate
//! consumes as its compiled-in registry trust anchor (§5.4
//! build-time-authoritative discipline).
//!
//! ## What lives here
//!
//! - The v1 lexicon JSON resources in `lexicons/`.
//! - `kryphocron-manifest.json` + `.kryphocron-manifest.lock` +
//!   `version.json` — the substrate-specific sidecar mechanism
//!   (§5.4 / §5.5).
//! - `build.rs` — the codegen + post-processing pipeline. Invokes
//!   `proto-blue-codegen` as a subprocess (§5.2 fallback path;
//!   §5.2's primary library-API path is blocked on proto-blue
//!   exporting a `lib.rs`, tracked as CHAINLINKS #17).
//! - [`Tier`], [`Visibility`], [`UnknownNsid`], [`SemVer`],
//!   [`DeprecationState`], [`LexiconRegistryEntry`] — the tier
//!   vocabulary, moved here from `kryphocron::tier` in Phase 2.
//!   See `PHASE_2_COMPLETION_REPORT.md` for the orphan-rules
//!   reasoning.
//! - The generated registry constant
//!   [`KRYPHOCRON_LEXICON_REGISTRY`] and the
//!   `impl Tier { pub fn from_nsid }` block, included from
//!   `OUT_DIR/registry.rs`.
//! - The generated `tools::kryphocron::*` module tree, mounted at
//!   `crate::tools::*` via `OUT_DIR/codegen-entry.rs`.
//!
//! ## What lives in `kryphocron`
//!
//! The type-system machinery built on top of [`Tier`] —
//! `TierWitness`, `PublicTier`, `PrivateTier`, `Tiered<T, Ti>`,
//! `MixedTier`, `HasNsid` — stays in `kryphocron` because the
//! envelope types are part of the substrate's threat-model
//! vocabulary, not the lexicon set.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc(html_no_source)]

// Re-export the proto-blue-syntax identifier types so consumers
// of kryphocron-lexicons can reach them via this crate. §5.7's
// lexicons reference these directly, and the generated codegen
// output qualifies them as `proto_blue_syntax::Did` etc.
pub use proto_blue_syntax::{AtUri, Datetime, Did, Handle, Nsid, RecordKey, Tid};

// Content-addressable identifier + blob reference types live in
// proto-blue-lex-data (one tier below proto-blue-lexicon's full
// LexiconDoc parser). Re-exported for consumers — kryphocron's
// §4.4 sensitive-representation layer and Phase 1's `Cid`
// placeholder both rest on `Cid` having one canonical home.
pub use proto_blue_lex_data::{BlobRef, Cid, CidError};

// Re-export the LexiconDoc shape so operator tooling can parse
// kryphocron lexicons without taking a separate dep on
// proto-blue-lexicon.
pub use proto_blue_lexicon::LexiconDoc;

// Tier vocabulary (moved here from kryphocron in Phase 2).
mod tier;
pub use tier::{
    DeprecationState, LexiconRegistryEntry, SemVer, Tier, UnknownNsid, Visibility,
};

// ---- Generated artifacts ----
//
// `codegen-entry.rs` mounts the proto-blue-codegen module tree at
// `crate::tools::*` via `#[path = "<abs>"] pub mod tools;`. The
// generated code uses qualified `crate::tools::...` references.
include!(concat!(env!("OUT_DIR"), "/codegen-entry.rs"));

// `registry.rs` defines `KRYPHOCRON_CODEGEN_HASH` (the §5.3
// hand-edit-rejection hash), `KRYPHOCRON_LEXICON_REGISTRY`, and
// the `impl Tier { pub fn from_nsid }` block.
include!(concat!(env!("OUT_DIR"), "/registry.rs"));
