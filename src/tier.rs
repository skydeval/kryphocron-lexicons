//! Tier vocabulary.
//!
//! Rust's orphan rules require the `impl Tier { fn from_nsid }`
//! that §5.3 commits to be in the same crate that defines [`Tier`].
//! The build-script-generated registry lives in
//! `kryphocron-lexicons`, so [`Tier`] (and the [`UnknownNsid`] error,
//! the registry-entry shape, and the [`DeprecationState`] enum) live
//! here too. The `kryphocron` crate re-exports them at its crate
//! root to preserve a single public surface for consumers.

use proto_blue_syntax::Nsid;
use thiserror::Error;

/// Tier classification for substrate-managed records.
///
/// See `kryphocron::tier::Tier` (re-exported) and §4.1 / §5.4.
/// `#[non_exhaustive]` from day one so future tier additions ship
/// as backward-compatible minor-version changes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Public-tier records. Visible to anyone.
    Public,
    /// Private-tier records. Visible only to authorized audiences.
    Private,
}

/// Result of a viewer-vs-tier visibility predicate (§4.1).
///
/// `Hidden` and `Forbidden` are distinct **internally** but
/// collapse to byte-identical wire responses at the HTTP layer
/// (§4.1 closed-namespace failure modes; §4.6 non-enumeration).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Viewer is authorized.
    Visible,
    /// Viewer is not in the resource's audience.
    Hidden,
    /// Viewer is policy-forbidden.
    Forbidden,
}

impl Visibility {
    /// True iff the viewer may read.
    #[must_use]
    pub fn allows_read(self) -> bool {
        matches!(self, Visibility::Visible)
    }
}

/// An NSID not present in the closed-namespace registry.
///
/// Returned by `Tier::from_nsid` when the supplied NSID is not in
/// `KRYPHOCRON_LEXICON_REGISTRY`. §4.1 closed-namespace failure
/// modes apply at the substrate ingress layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum UnknownNsid {
    /// The NSID was not present in the closed-namespace registry.
    #[error("NSID `{0}` is not present in the closed-namespace registry")]
    NotRegistered(Nsid),
}

/// Semver triplet used in deprecation state (§5.6).
///
/// Sited here (rather than in the `kryphocron` crate) so the
/// build-script-generated registry constant can reference it.
/// The `kryphocron` crate re-exports this at its crate root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemVer {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

impl SemVer {
    /// Construct a [`SemVer`] from major/minor/patch.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        SemVer { major, minor, patch }
    }
}

/// Per-NSID deprecation state (§5.6).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeprecationState {
    /// Active; writes and reads both proceed normally.
    Active,
    /// Deprecated; writes rejected at §4.3 stage 0; reads proceed.
    Deprecated {
        /// Lexicon-set version the deprecation landed in.
        since_version: SemVer,
        /// Successor NSID, if committed.
        successor: Option<&'static str>,
    },
    /// Deprecated but inside an operator-configured grace window.
    /// Writes proceed and emit a `DeprecatedWriteDuringGrace` audit
    /// event; switches to `Deprecated` after `grace_until`.
    DeprecatedWithGrace {
        /// Lexicon-set version the deprecation landed in.
        since_version: SemVer,
        /// When grace ends (encoded as UTC seconds since UNIX epoch
        /// to keep the const evaluable; the kryphocron crate
        /// converts to `SystemTime` at the stage-0 check).
        grace_until_unix_seconds: i64,
        /// Successor NSID, if committed.
        successor: Option<&'static str>,
    },
}

/// One entry in `KRYPHOCRON_LEXICON_REGISTRY` (§5.3 / §5.6).
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct LexiconRegistryEntry {
    /// NSID of the lexicon.
    pub nsid: &'static str,
    /// Tier classification (immutable per §5.5).
    pub tier: Tier,
    /// Current deprecation state.
    pub deprecation: DeprecationState,
}
