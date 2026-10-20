# The lexicon set

The full JSON is under `lexicons/tools/kryphocron/`. This document
explains what each lexicon means and what it commits to.

## Records

| NSID | Tier | Audience rule |
| --- | --- | --- |
| `feed.postPublic` | Public | n/a |
| `feed.postPrivate` | Private | declares `audienceList` |
| `feed.like` | Public | n/a |
| `feed.repost` | Public | n/a |
| `feed.threadgate` | Public | n/a |
| `graph.block` | Private | exempt (block oracle) |
| `graph.mute` | Private | exempt (mute oracle) |
| `policy.audience` | Private | exempt (the audience mechanism itself) |

All NSIDs are under `tools.kryphocron.`, and every record uses `tid`
keys.

### `feed.postPublic`

A public post. Its field schema matches `app.bsky.feed.post`: `text`,
`createdAt`, `reply`, `embed` (image, external link, quoted record)
and `facets`. An AppView can parse it with the parsers it already has
for `app.bsky.feed.post`.

The compatibility covers the fields only, not indexing. AppViews
filter records by NSID, so a stock AppView that indexes only
`app.bsky.feed.post` will not pick these records up. An AppView has
to be configured to index this NSID.

`postPublic.json` also hosts the defs that both posts share: the
reply, strong-ref, embed, facet and label shapes. `postPrivate`
references them by `tools.kryphocron.feed.postPublic#<name>`. Adding
a shared def adds no NSID.

The substrate emits these posts to the public firehose and never
consults an audience list for them.

### `feed.postPrivate`

A private post. The substrate serves it only to requesters the
audience it names admits. That is access friction, not encryption:
the record can still reach sync channels such as the firehose.

| Field | |
| --- | --- |
| `audienceList` | **Required.** The AT-URI of a `policy.audience` record. Its presence is what makes the post private: a post without an audience is a public post, so use `postPublic`. |
| `text` | Post text (max 300 graphemes). |
| `encodedContent`, `encodedContentCodec`, `encodedContentGeneration` | Output of an optional at-rest content codec, used in place of `text`, plus the codec's identifier and a rotation-generation mark. The substrate does not interpret the codec output. |
| `embedBlobs`, `mentionDids`, `quote` | Set by an encoding host when `encodedContent` carries the embed and facets: the embed's blobs (so the host keeps them), the mentioned DIDs (deduplicated, first-appearance order) and a strong ref to the quoted record. They stay in the clear so AppViews can index mentions and quotes without decoding. Derived from the encoded content; the decoded content is authoritative. |
| `mediaKinds` | Set by an encoding host when `encodedContent` carries the embed: `image`, `video` or both (in that order, each once) for the media the post's own embed carries, so AppViews can filter an author's posts by media without decoding. A record-with-media embed counts its own media; link cards, plain quotes and a quoted post's media do not. Absent when there is no media. |
| `createdAt` | **Required.** |
| `reply`, `facets` | As on `postPublic`. |
| `embed` | One of `#embedImage` (up to 4), `#embedGallery` (up to 20), `#embedVideo`, `#embedExternal`, `#embedRecord`, `#embedRecordWithMedia` (a quoted record plus images, a gallery, a video or a link card). |
| `langs` | Up to three language tags. |
| `labels` | Self-applied content labels (`#selfLabels`). |
| `publicCompanion` | The AT-URI of a paired public-tier record, when the post has a public side. Only the private record carries this link. |

**Each record must carry exactly one of `text` or `encodedContent`.**
JSON Schema cannot express that rule, so the host enforces it, not
the lexicon. A post with only media sends `"text": ""`.

The embed, label and aspect-ratio shapes mirror their `app.bsky.*`
counterparts. That lets a reader map them onto the stock view types.
Two differences from `#embedImage` matter to writers of
`#embedGallery`: every gallery item is a typed union member (with
`$type` `…#galleryImage`), and `aspectRatio` is required on gallery
images.

Read access goes through the audience oracle at read time. Emitting a
private post to a sync channel needs its own capability.

### `feed.like`, `feed.repost`

`subject` (a strong ref) and `createdAt`. They are shaped like
`app.bsky.feed.like` and `app.bsky.feed.repost`, and are always
public tier.

A public like or repost of a private post reveals that the
relationship exists, even though the post's content stays gated. The
lexicon layer sees `subject` only as an opaque reference, so this
crate does not prevent such records. Whether to allow them is a
product decision.

### `feed.threadgate`

`post` (a strong ref), `allow` (mention, following and list rules)
and `createdAt`. It is shaped like `app.bsky.feed.threadgate`. On a
private thread, its rules apply **in addition to** audience
membership: a reply needs both.

### `graph.block`, `graph.mute`

`subject` (a DID) and `createdAt`. The record's owner is the one
blocking or muting. Both are private tier, but that does not make
them confidential: like any repo record they can reach sync channels
such as the firehose, so who blocked or muted whom is observable
there.

Whether a block record is visible is separate from how blocks are
enforced. Blocks are enforced in both directions by kryphocron's
block oracle, whatever the record's visibility.

### `policy.audience`

An audience list: who may read the private records that reference
it. Owned by the account that writes it.

| Field | |
| --- | --- |
| `mode` | `list`, `everyone`, `followers`, `following` or `nobody`. An absent mode reads as `list`. These are `knownValues` (open) in the lexicon; the host enforces the closed set. |
| `members` | Authorized DIDs. The host requires this when `mode` is `list`. |
| `name` | A label for display only. It plays no part in authorization. |
| `createdAt`, `updatedAt` | **Required.** |

A resource references exactly one audience.

**Membership changes are retroactive.** The audience is read when a
record is read, not when it was written. Adding a member grants
access to every past record that names the list. Removing a member
stops any further reads through the substrate. Copies the removed
member already made cannot be recalled. A change takes effect within
the audience oracle's freshness bound, not instantly.

A product that wants an audience frozen when a post is written must
build that itself, for example by versioning lists or by pointing
each post at a snapshot.

The audience oracle reads these records **before** the requester is
authorized. Routing those reads through the ordinary read capability
would be circular, so the oracle's access runs under implicit
substrate-class trust. Protect the storage behind the oracle
accordingly.

## Procedures

These describe the XRPC endpoints a host exposes for private-tier
writes. They carry no tier and get no Rust codegen (see
[build-pipeline.md](build-pipeline.md)).

| NSID | Input | Output |
| --- | --- | --- |
| `feed.createPostPrivate` | `repo`, `rkey?`, `validate?`, `record` (a `postPrivate`) | `uri`, `cid` |
| `feed.updatePostPrivate` | `repo`, `rkey`, `audienceList`, `swapRecord?` | `uri`, `cid` |
| `feed.deletePostPrivate` | `repo`, `rkey` | empty |
| `actor.participatePrivate` | `repo`, `rkey?`, `validate?`, `record` (a `postPrivate` reply) | `uri`, `cid` |
| `policy.manageAudience` | `repo`, `rkey?`, `validate?`, `swapRecord?`, `record` (a `policy.audience`) | `uri`, `cid` |
| `policy.deleteAudience` | `repo`, `rkey` | empty |

- **`repo`** is the writer's DID and must be the authenticated
  account.
- **`cid`** is the CID of the written record.

### Each procedure's rules

- **`createPostPrivate`:** the post may name only the writer's own
  audiences.
- **`participatePrivate`:** writes a reply under another account's
  audience, and checks the writer against that audience first. Every
  audience refusal gets the same `Forbidden`, so a refusal reveals
  nothing about the audience.
- **`updatePostPrivate`:** changes only the post's `audienceList`.
  The URI stays the same and the CID changes. Replies are refused,
  since a reply's audience is its thread's.
- **`manageAudience`:** updates the record when given the `rkey` of
  an existing one, and creates one otherwise.
- **`deleteAudience`:** after deletion, posts that still name the
  audience are readable by their authors only.

### Errors

- `InvalidRequest`: a malformed request, or one that breaks a
  procedure's rule.
- `Forbidden`: the caller may not write here, or the audience check
  denied the write.
- `UnsupportedNamespace`: the host does not have the private tier
  enabled.
- `BlobNotFound`: an embed references a blob the host does not hold.
- `InvalidSwap`: `swapRecord` did not match the current record.
