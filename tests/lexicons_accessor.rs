//! Tests for the runtime `lexicons()` accessor.
//!
//! - `lexicons_accessor_loads_all_registered_nsids` is the smoke test
//!   that guards the build script: every NSID in
//!   `KRYPHOCRON_LEXICON_REGISTRY` must also be present in the
//!   `lexicons()` collection. A future registry entry added without a
//!   corresponding lexicon JSON in `LEXICON_JSONS` fails here.
//! - `validate_record_*` exercise the end-to-end wiring a write-path
//!   consumer relies on: take the `&'static Lexicons` from
//!   `lexicons()`, pull the `postPrivate` record def out of it, and run
//!   `proto_blue_lexicon::validate_record` against a value.

use std::collections::BTreeMap;

use proto_blue_lex_data::LexValue;
use proto_blue_lexicon::{validate_record, LexRecord, LexUserType};

#[test]
fn lexicons_accessor_loads_all_registered_nsids() {
    let lexicons = kryphocron_lexicons::lexicons();
    for entry in kryphocron_lexicons::KRYPHOCRON_LEXICON_REGISTRY {
        assert!(
            lexicons.get(entry.nsid).is_some(),
            "lexicon {} present in registry but not in lexicons() accessor",
            entry.nsid
        );
    }
}

/// Pull the `postPrivate` record definition out of the live collection.
fn post_private_record() -> &'static LexRecord {
    let lexicons = kryphocron_lexicons::lexicons();
    let def = lexicons
        .get_def("tools.kryphocron.feed.postPrivate#main")
        .expect("postPrivate#main must resolve");
    match def {
        LexUserType::Record(rec) => rec,
        other => panic!("postPrivate#main should be a record, got {}", other.type_name()),
    }
}

/// A conformant `audienceList` value. The field is an at-uri string
/// referencing a `policy.audience` record (consulted at read time),
/// not an embedded object.
fn audience_value() -> LexValue {
    LexValue::from("at://did:plc:z72i7hdynmk6r22z27h6tvur/tools.kryphocron.policy.audience/3kaudiencelist01")
}

#[test]
fn validate_record_accepts_valid_post_private() {
    let lexicons = kryphocron_lexicons::lexicons();
    let rec = post_private_record();

    let mut m: BTreeMap<String, LexValue> = BTreeMap::new();
    m.insert("text".to_string(), LexValue::from("a private post"));
    m.insert("createdAt".to_string(), LexValue::from("2026-05-31T12:30:00Z"));
    m.insert("audienceList".to_string(), audience_value());
    let value = LexValue::from(m);

    assert!(
        validate_record(lexicons, rec, &value).is_ok(),
        "expected a fully-populated postPrivate value to validate"
    );
}

#[test]
fn validate_record_rejects_missing_required_field() {
    let lexicons = kryphocron_lexicons::lexicons();
    let rec = post_private_record();

    // `audienceList` is required by postPrivate#main; omitting it must fail.
    // (`text` is optional as of 0.3.0 — a record may carry `encodedContent`
    // instead — so its absence is no longer a validation failure.)
    let mut m: BTreeMap<String, LexValue> = BTreeMap::new();
    m.insert("text".to_string(), LexValue::from("a private post"));
    m.insert("createdAt".to_string(), LexValue::from("2026-05-31T12:30:00Z"));
    let value = LexValue::from(m);

    assert!(
        validate_record(lexicons, rec, &value).is_err(),
        "expected a postPrivate value missing the required `audienceList` field to fail validation"
    );
}
