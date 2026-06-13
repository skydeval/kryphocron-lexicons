//! Invariant tests for the kryphocron-lexicons companion crate.
//!
//! Aim: invariant verification, not coverage. This file pins
//! the structural properties §5.3 / §5.4 / §5.7 commit:
//!
//! - **Round-trip parsing**: every v1 lexicon JSON in
//!   `lexicons/` deserializes as a `LexiconDoc` per
//!   `proto-blue-lexicon`. Catches schema drift in either the
//!   committed lexicon JSON or the upstream parser.
//! - **Registry pinning**: `KRYPHOCRON_LEXICON_REGISTRY` contains
//!   exactly the eight v1 NSIDs in §5.7's tier classifications.
//!   Catches accidental tier changes (which the lockfile would
//!   also catch at build time, but pinning here is a public-
//!   surface contract).
//! - **`Tier::from_nsid` pinning**: each of the eight v1 NSIDs
//!   resolves to its committed tier; unknown NSIDs return
//!   `Err(UnknownNsid::NotRegistered)`. The closed-namespace
//!   property of §4.1 is structural here.
//! - **Audience-ref structural rule sanity**: every private-tier
//!   lexicon either declares the `audienceList` at-uri reference
//!   field or is marked `audience_exempt` in the manifest. This test
//!   shadows the build-script check (§5.4) for an outside-the-
//!   build-pipeline second opinion.
//!
//! Build-script **failure-mode** tests (drift between manifest /
//! lexicons / codegen output producing specific errors; lockfile
//! immutability violations producing specific errors; private-
//! tier audience-list missing producing the §5.4 error;
//! hash-based hand-edit rejection) are exercised via manual
//! reproductions documented separately, not in this file —
//! triggering them requires committing a deliberately-broken
//! manifest, which is unsuited to an in-tree test.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use kryphocron_lexicons::{
    KRYPHOCRON_LEXICON_REGISTRY, LexiconDoc, Nsid, Tier, UnknownNsid,
};

/// The eight v1 NSIDs and their §5.4-committed tiers.
fn v1_nsids() -> Vec<(&'static str, Tier)> {
    vec![
        ("tools.kryphocron.feed.postPublic", Tier::Public),
        ("tools.kryphocron.feed.postPrivate", Tier::Private),
        ("tools.kryphocron.feed.like", Tier::Public),
        ("tools.kryphocron.feed.repost", Tier::Public),
        ("tools.kryphocron.feed.threadgate", Tier::Public),
        ("tools.kryphocron.graph.block", Tier::Private),
        ("tools.kryphocron.graph.mute", Tier::Private),
        ("tools.kryphocron.policy.audience", Tier::Private),
    ]
}

fn lexicons_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lexicons")
}

fn lexicon_path(nsid: &str) -> PathBuf {
    let mut p = lexicons_dir();
    for seg in nsid.split('.') {
        p = p.join(seg);
    }
    p.set_extension("json");
    p
}

#[test]
fn every_v1_lexicon_parses_as_lexicon_doc() {
    for (nsid, _) in v1_nsids() {
        let path = lexicon_path(nsid);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let doc: LexiconDoc = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {nsid}: {e}"));
        assert_eq!(doc.id, nsid, "id field on {nsid} matches NSID");
    }
}

#[test]
fn registry_contains_exactly_the_v1_nsids() {
    let expected: BTreeMap<&str, Tier> = v1_nsids().into_iter().collect();
    let actual: BTreeMap<&str, Tier> = KRYPHOCRON_LEXICON_REGISTRY
        .iter()
        .map(|e| (e.nsid, e.tier))
        .collect();
    assert_eq!(
        actual.keys().copied().collect::<Vec<_>>(),
        expected.keys().copied().collect::<Vec<_>>(),
        "registry NSID set",
    );
    for (nsid, tier) in &expected {
        assert_eq!(actual.get(nsid).copied(), Some(*tier), "{nsid} tier");
    }
}

#[test]
fn registry_size_pinned_at_eight() {
    assert_eq!(
        KRYPHOCRON_LEXICON_REGISTRY.len(),
        8,
        "v1 baseline lexicon set is eight entries (§5.7)",
    );
}

#[test]
fn tier_from_nsid_resolves_every_v1_lexicon() {
    for (nsid_str, expected_tier) in v1_nsids() {
        let nsid = Nsid::new(nsid_str).unwrap();
        let resolved = Tier::from_nsid(&nsid)
            .unwrap_or_else(|_| panic!("{nsid_str} not registered"));
        assert_eq!(resolved, expected_tier, "{nsid_str}");
    }
}

#[test]
fn tier_from_nsid_rejects_unknown_nsid() {
    // Any NSID not in the registry must return NotRegistered.
    let unknown = Nsid::new("com.example.not.in.registry").unwrap();
    let result = Tier::from_nsid(&unknown);
    assert!(
        matches!(result, Err(UnknownNsid::NotRegistered(_))),
        "{result:?}",
    );
}

#[test]
fn tier_from_nsid_rejects_lookalike_with_wrong_authority() {
    // An NSID that uses `tools.kryphocron.*`-shaped naming but is
    // not committed to the registry — closed-namespace failure.
    let pretender =
        Nsid::new("tools.kryphocron.feed.notARealLexicon").unwrap();
    assert!(matches!(
        Tier::from_nsid(&pretender),
        Err(UnknownNsid::NotRegistered(_)),
    ));
}

#[test]
fn private_tier_lexicons_satisfy_section_5_4_rule() {
    // Each private-tier lexicon either:
    // (a) is `tools.kryphocron.policy.audience` itself (the
    //     audience mechanism), or
    // (b) declares an `audienceList` at-uri reference field, or
    // (c) is `audience_exempt` per the manifest (verified at
    //     build time; this test shadows the structural check by
    //     re-reading the lexicon JSON).
    //
    // The build script enforces this on every build (§5.4); this
    // is a second-opinion shadow check that catches a future
    // commit that toggles a private lexicon to non-exempt without
    // adding the audienceList at-uri reference.
    let manifest_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kryphocron-manifest.json");
    let manifest_raw = fs::read_to_string(&manifest_path).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_raw).unwrap();

    for (nsid_str, tier) in v1_nsids() {
        if tier != Tier::Private {
            continue;
        }
        let entry = manifest
            .pointer(&format!("/lexicons/{}", nsid_str.replace('/', "~1")))
            .unwrap_or_else(|| panic!("manifest missing entry for {nsid_str}"));
        let audience_exempt = entry
            .get("audience_exempt")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if audience_exempt {
            // Must have an exemption_reason per §5.4.
            assert!(
                entry
                    .get("exemption_reason")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty()),
                "{nsid_str}: audience_exempt requires non-empty exemption_reason",
            );
            continue;
        }

        // Not exempt — verify the lexicon JSON declares the
        // audienceList ref.
        let raw = fs::read_to_string(lexicon_path(nsid_str)).unwrap();
        let doc: LexiconDoc = serde_json::from_str(&raw).unwrap();
        let main = doc.defs.get("main").expect("main def exists");
        use proto_blue_lexicon::types::{LexObject, LexUserType};
        let object: &LexObject = match main {
            LexUserType::Record(r) => &r.record,
            LexUserType::Object(o) => o,
            _ => panic!("{nsid_str}: main is neither record nor object"),
        };
        let has_audience_ref = matches!(
            object.properties.get("audienceList"),
            Some(LexUserType::String(s)) if s.format.as_deref() == Some("at-uri")
        );
        assert!(
            has_audience_ref,
            "{nsid_str}: private-tier without an `audienceList` at-uri reference must be `audience_exempt`",
        );
    }
}

#[test]
fn codegen_hash_is_a_64_char_lowercase_hex_string() {
    use kryphocron_lexicons::KRYPHOCRON_CODEGEN_HASH;
    assert_eq!(
        KRYPHOCRON_CODEGEN_HASH.len(),
        64,
        "SHA-256 digest renders as 64 hex chars",
    );
    assert!(
        KRYPHOCRON_CODEGEN_HASH
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "hex chars only, no uppercase: {KRYPHOCRON_CODEGEN_HASH}",
    );
}
