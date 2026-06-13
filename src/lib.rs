//! # kryphocron-lexicons — companion lexicon crate
//!
//! Companion crate to [kryphocron]. Ships the eight v1 ATProto
//! lexicons under `tools.kryphocron.*`, the substrate-specific
//! sidecar manifest, the build-time three-way registry
//! consistency check, and the [`KRYPHOCRON_LEXICON_REGISTRY`]
//! constant the kryphocron crate consumes as its compiled-in,
//! build-time-authoritative registry trust anchor.
//!
//! [kryphocron]: https://crates.io/crates/kryphocron
//!
//! ## What lives here
//!
//! - The v1 lexicon JSON resources in `lexicons/`.
//! - `kryphocron-manifest.json` + `.kryphocron-manifest.lock` +
//!   `version.json` — the substrate-specific sidecar mechanism.
//! - `build.rs` — the codegen + post-processing pipeline. Invokes
//!   `proto-blue-codegen` as a subprocess; the primary
//!   library-API path is blocked on proto-blue exporting a
//!   `lib.rs`.
//! - [`Tier`], [`Visibility`], [`UnknownNsid`], [`SemVer`],
//!   [`DeprecationState`], [`LexiconRegistryEntry`] — the tier
//!   vocabulary. Rust's orphan rules require
//!   `impl Tier { fn from_nsid }` to live in the same crate that
//!   defines [`Tier`]; the registry is build-script-generated
//!   here, so [`Tier`] lives here too.
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
// of kryphocron-lexicons can reach them via this crate. The
// lexicons reference these directly, and the generated codegen
// output qualifies them as `proto_blue_syntax::Did` etc.
pub use proto_blue_syntax::{AtUri, Datetime, Did, Handle, Nsid, RecordKey, Tid};

// Content-addressable identifier + blob reference types live in
// proto-blue-lex-data (one tier below proto-blue-lexicon's full
// LexiconDoc parser). Re-exported for consumers — kryphocron's
// sensitive-representation layer rests on `Cid` having one
// canonical home.
pub use proto_blue_lex_data::{BlobRef, Cid, CidError};

// Re-export the LexiconDoc shape so operator tooling can parse
// kryphocron lexicons without taking a separate dep on
// proto-blue-lexicon.
pub use proto_blue_lexicon::LexiconDoc;

// Tier vocabulary (sited here to satisfy orphan rules for the
// build-script-generated `impl Tier { fn from_nsid }`).
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

// `registry.rs` defines `KRYPHOCRON_CODEGEN_HASH` (the
// hand-edit-rejection hash), `KRYPHOCRON_LEXICON_REGISTRY`, and
// the `impl Tier { pub fn from_nsid }` block.
include!(concat!(env!("OUT_DIR"), "/registry.rs"));

// `lexicon_jsons.rs` defines `LEXICON_JSONS: &[(&str, &str)]` — the
// verbatim lexicon JSON text paired with each NSID, emitted by the
// build script. Consumed by the `lexicons()` accessor below.
include!(concat!(env!("OUT_DIR"), "/lexicon_jsons.rs"));

// ---- Runtime lexicon-document accessor ----

use std::sync::OnceLock;

use proto_blue_lexicon::Lexicons;

static LEXICONS: OnceLock<Lexicons> = OnceLock::new();

/// Returns the full set of `tools.kryphocron.*` lexicon documents as
/// a proto-blue [`Lexicons`] collection, suitable for use with
/// `proto_blue_lexicon::validate_record` and the other AST-shaped
/// validators.
///
/// The metadata-only [`KRYPHOCRON_LEXICON_REGISTRY`] exposes each
/// lexicon's NSID, tier, and deprecation state; this accessor exposes
/// the underlying parsed schema documents, which validation needs and
/// the registry does not carry. The two are complementary, as are the
/// codegen `tools::*` typed structs for typed Rust access.
///
/// Constructed once per process via [`OnceLock`]: the first call parses
/// every embedded JSON and builds the collection; subsequent calls
/// return the same `&'static Lexicons`.
///
/// # Panics
///
/// Panics if any embedded lexicon JSON fails to parse or fails
/// proto-blue's load-time schema refinement. The JSON is vendored
/// in-tree and embedded at build time, so a failure here means the
/// crate itself is broken, not that a caller supplied bad input — a
/// loud panic is the correct response.
#[must_use]
pub fn lexicons() -> &'static Lexicons {
    LEXICONS.get_or_init(|| {
        let mut docs = Lexicons::new();
        for (nsid, json) in LEXICON_JSONS {
            docs.add_from_json(json).unwrap_or_else(|e| {
                panic!("kryphocron-lexicons: failed to load lexicon doc for {nsid}: {e}")
            });
        }
        docs
    })
}
