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

// ---- Media embeds, langs, self-labels ----
//
// The embed and labels unions are open, so a `$type` the validator
// cannot resolve passes silently. Each accept case below is paired with
// a reject case that violates a constraint *inside* the referenced def,
// proving the def resolved and was applied rather than skipped.

fn obj(entries: &[(&str, LexValue)]) -> LexValue {
    LexValue::from(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect::<BTreeMap<String, LexValue>>(),
    )
}

fn blob(mime: &str, size: i64) -> LexValue {
    let cid: proto_blue_lex_data::Cid = "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"
        .parse()
        .expect("test CID parses");
    obj(&[
        ("$type", LexValue::from("blob")),
        ("ref", LexValue::from(cid)),
        ("mimeType", LexValue::from(mime)),
        ("size", LexValue::from(size)),
    ])
}

fn aspect(width: i64, height: i64) -> LexValue {
    obj(&[
        ("width", LexValue::from(width)),
        ("height", LexValue::from(height)),
    ])
}

fn strong_ref() -> LexValue {
    obj(&[
        (
            "uri",
            LexValue::from("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3kquotedpost01"),
        ),
        (
            "cid",
            LexValue::from("bafyreie5737gdxlw5i64vzichcalba3z2v5n6icifvx5xytvske7mr3hpm"),
        ),
    ])
}

fn video_embed(mime: &str) -> LexValue {
    obj(&[
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#embedVideo")),
        ("video", blob(mime, 5_000_000)),
        ("alt", LexValue::from("a clip")),
        ("aspectRatio", aspect(16, 9)),
        ("presentation", LexValue::from("default")),
        (
            "captions",
            LexValue::from(vec![obj(&[
                ("lang", LexValue::from("en")),
                ("file", blob("text/vtt", 1_000)),
            ])]),
        ),
    ])
}

fn images_embed(image_size: i64, aspect_width: i64) -> LexValue {
    obj(&[
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#embedImage")),
        (
            "images",
            LexValue::from(vec![obj(&[
                ("image", blob("image/jpeg", image_size)),
                ("alt", LexValue::from("a photo")),
                ("aspectRatio", aspect(aspect_width, 3)),
            ])]),
        ),
    ])
}

fn gallery_item(mime: &str, with_aspect: bool) -> LexValue {
    let mut entries = vec![
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#galleryImage")),
        ("image", blob(mime, 1_500_000)),
        ("alt", LexValue::from("a photo")),
    ];
    if with_aspect {
        entries.push(("aspectRatio", aspect(4, 3)));
    }
    obj(&entries)
}

fn gallery_embed(items: Vec<LexValue>) -> LexValue {
    obj(&[
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#embedGallery")),
        ("items", LexValue::from(items)),
    ])
}

fn gallery_of(n: usize) -> LexValue {
    gallery_embed((0..n).map(|_| gallery_item("image/jpeg", true)).collect())
}

fn external_embed(thumb_mime: &str) -> LexValue {
    obj(&[
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#embedExternal")),
        (
            "external",
            obj(&[
                ("uri", LexValue::from("https://example.com/article")),
                ("title", LexValue::from("An article")),
                ("description", LexValue::from("About things")),
                ("thumb", blob(thumb_mime, 50_000)),
            ]),
        ),
    ])
}

fn record_with_media(media: LexValue) -> LexValue {
    obj(&[
        (
            "$type",
            LexValue::from("tools.kryphocron.feed.postPublic#embedRecordWithMedia"),
        ),
        (
            "record",
            obj(&[
                ("$type", LexValue::from("tools.kryphocron.feed.postPublic#embedRecord")),
                ("record", strong_ref()),
            ]),
        ),
        ("media", media),
    ])
}

fn self_labels(val: &str) -> LexValue {
    obj(&[
        ("$type", LexValue::from("tools.kryphocron.feed.postPublic#selfLabels")),
        ("values", LexValue::from(vec![obj(&[("val", LexValue::from(val))])])),
    ])
}

/// A media-only private post: `text` is the empty string, satisfying the
/// substrate-side text-XOR-encodedContent rule.
fn post_with(extra: &[(&str, LexValue)]) -> LexValue {
    let mut entries = vec![
        ("text", LexValue::from("")),
        ("createdAt", LexValue::from("2026-10-15T12:00:00Z")),
        ("audienceList", audience_value()),
    ];
    entries.extend(extra.iter().cloned());
    obj(&entries)
}

fn validates(value: &LexValue) -> bool {
    validate_record(kryphocron_lexicons::lexicons(), post_private_record(), value).is_ok()
}

#[test]
fn video_embed_validates_and_its_blob_constraints_apply() {
    assert!(validates(&post_with(&[("embed", video_embed("video/mp4"))])));
    assert!(
        !validates(&post_with(&[("embed", video_embed("image/png"))])),
        "embedVideo must resolve: a non-mp4 video blob has to be rejected"
    );
}

#[test]
fn image_embed_carries_aspect_ratio_and_two_megabyte_limit() {
    assert!(validates(&post_with(&[("embed", images_embed(1_500_000, 4))])));
    assert!(
        !validates(&post_with(&[("embed", images_embed(2_000_001, 4))])),
        "image blob over 2 MB must be rejected"
    );
    assert!(
        !validates(&post_with(&[("embed", images_embed(1_500_000, 0))])),
        "aspectRatio must resolve: width 0 is below the minimum of 1"
    );
}

#[test]
fn gallery_embed_holds_up_to_twenty_images_with_required_aspect_ratio() {
    assert!(validates(&post_with(&[("embed", gallery_of(10))])));
    assert!(validates(&post_with(&[("embed", gallery_of(20))])));
    assert!(
        !validates(&post_with(&[("embed", gallery_of(21))])),
        "gallery over 20 items must be rejected"
    );
    assert!(
        !validates(&post_with(&[(
            "embed",
            gallery_embed(vec![gallery_item("image/jpeg", false)])
        )])),
        "galleryImage requires aspectRatio"
    );
    assert!(
        !validates(&post_with(&[(
            "embed",
            gallery_embed(vec![gallery_item("video/mp4", true)])
        )])),
        "the item union must resolve: a non-image gallery item has to be rejected"
    );
}

#[test]
fn external_embed_carries_thumb() {
    assert!(validates(&post_with(&[("embed", external_embed("image/png"))])));
    assert!(
        !validates(&post_with(&[("embed", external_embed("video/mp4"))])),
        "thumb must resolve: a non-image thumb has to be rejected"
    );
}

#[test]
fn record_with_media_validates_each_media_kind() {
    for media in [
        images_embed(1_500_000, 4),
        gallery_of(10),
        video_embed("video/mp4"),
        external_embed("image/png"),
    ] {
        assert!(validates(&post_with(&[("embed", record_with_media(media))])));
    }
    assert!(
        !validates(&post_with(&[(
            "embed",
            record_with_media(video_embed("image/png"))
        )])),
        "the media union must resolve: an invalid video inside a quote has to be rejected"
    );
    assert!(
        !validates(&post_with(&[(
            "embed",
            record_with_media(images_embed(1_500_000, 0))
        )])),
        "the media union must resolve: an invalid image inside a quote has to be rejected"
    );
    assert!(
        !validates(&post_with(&[(
            "embed",
            record_with_media(gallery_embed(vec![gallery_item("image/jpeg", false)]))
        )])),
        "the media union must resolve: an invalid gallery inside a quote has to be rejected"
    );
}

#[test]
fn langs_are_bounded_to_three_language_tags() {
    let langs = |tags: &[&str]| {
        LexValue::from(tags.iter().map(|t| LexValue::from(*t)).collect::<Vec<_>>())
    };
    assert!(validates(&post_with(&[("langs", langs(&["en", "pt-BR"]))])));
    assert!(!validates(&post_with(&[("langs", langs(&["en", "de", "fr", "ja"]))])));
    assert!(!validates(&post_with(&[("langs", langs(&["not a language tag"]))])));
}

#[test]
fn self_labels_validate_and_bound_value_length() {
    assert!(validates(&post_with(&[("labels", self_labels("graphic-media"))])));
    assert!(
        !validates(&post_with(&[("labels", self_labels(&"x".repeat(129)))])),
        "selfLabels must resolve: a 129-byte label value has to be rejected"
    );
}

#[test]
fn clear_index_fields_validate_and_their_types_apply() {
    let dids = |ds: &[&str]| {
        LexValue::from(ds.iter().map(|d| LexValue::from(*d)).collect::<Vec<_>>())
    };
    let blobs = LexValue::from(vec![blob("image/jpeg", 1_500_000), blob("video/mp4", 5_000_000)]);
    assert!(validates(&post_with(&[
        ("embedBlobs", blobs),
        ("mentionDids", dids(&["did:plc:z72i7hdynmk6r22z27h6tvur", "did:web:example.com"])),
        ("quote", strong_ref()),
    ])));

    let not_blobs = LexValue::from(vec![LexValue::from("not a blob")]);
    assert!(
        !validates(&post_with(&[("embedBlobs", not_blobs)])),
        "embedBlobs items must be blobs"
    );
    assert!(
        !validates(&post_with(&[("mentionDids", dids(&["alice.example.com"]))])),
        "mentionDids items must be DIDs"
    );
    let cidless = obj(&[(
        "uri",
        LexValue::from("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3kquotedpost01"),
    )]);
    assert!(
        !validates(&post_with(&[("quote", cidless)])),
        "quote must resolve to strongRef: a ref without a cid has to be rejected"
    );
}

#[test]
fn media_kinds_validate_and_are_bounded() {
    let kinds = |ks: &[&str]| {
        LexValue::from(ks.iter().map(|k| LexValue::from(*k)).collect::<Vec<_>>())
    };
    assert!(validates(&post_with(&[("mediaKinds", kinds(&["image"]))])));
    assert!(validates(&post_with(&[("mediaKinds", kinds(&["image", "video"]))])));
    assert!(
        !validates(&post_with(&[("mediaKinds", kinds(&["image", "video", "image"]))])),
        "mediaKinds must be bounded: a third item has to be rejected"
    );
    assert!(
        !validates(&post_with(&[("mediaKinds", LexValue::from(vec![LexValue::from(1i64)]))])),
        "mediaKinds items must be strings"
    );
}
