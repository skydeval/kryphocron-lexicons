# Versioning and deprecation

## Three version numbers

| Version | Where | What it versions |
| --- | --- | --- |
| Crate version | `Cargo.toml` | The Rust crate: its API, the generated types, the build behavior. |
| Lexicon-set version | `version.json` | The wire vocabulary as a whole. |
| Per-lexicon version | `lexicon_version` in the manifest | One lexicon's own revisions. |

`version.json` also records `compatibility.min_substrate_version`,
the oldest kryphocron the lexicon set supports. The build reads the
lexicon-set version for one purpose: it stamps
`first_seen_in_lexicon_set_version` on newly locked lexicons. The
crate exposes no runtime constant for it. kryphocron hosts configure
their own lexicon-set version for the sync handshake (below).

### Crate releases

The crate follows semver with the usual Rust 0.x convention: a 0.x
minor release may break, and a patch release may not. Generated
structs and enums are not `#[non_exhaustive]`. Adding a field or a
union member is therefore source-breaking for Rust code that builds
those types with struct literals or matches them exhaustively.
Deserializing and reading fields are unaffected. The CHANGELOG says
when a release does this.

## Compatibility rules for the wire vocabulary

The lexicon set is versioned like a Rust crate. While it is 0.x, a
breaking wire change bumps the minor version and an addition bumps
the patch version. Up to now the lexicon-set version has followed the
crate version; the two may diverge in future, for example when a
crate release changes only Rust code.

From 1.0.0 on, these rules apply.

**Always allowed:**

- adding a lexicon;
- adding an optional field;
- adding a member to an open union, or a value to an open
  `knownValues` set.

**Not allowed without a major bump of the lexicon set:**

- removing a lexicon;
- removing a field;
- changing an existing field's type;
- deprecating a lexicon.

**Never allowed, in any version:** reclassifying a tier.

Unions and `knownValues` are open, so a consumer must accept members
and values it does not recognize.

## Version skew between peers

ATProto records carry no lexicon version, and the substrate does not
try to infer one from each record. Instead, kryphocron compares
lexicon-set versions at the sync handshake. Each side states its
version, and a peer on a newer **major** version than the local host
is refused before any record is exchanged:

| Local vs. peer | Result |
| --- | --- |
| Older local, newer major on the peer | Sync refused at the handshake (`LexiconSetMajorVersionMismatch`). |
| Newer local, older peer | Sync proceeds. The older peer simply never sends lexicons it does not know. |
| Same major version | Normal operation. |

**While the lexicon set is 0.x, this check cannot refuse anything.**
Every 0.x peer has major version 0, so a peer with a breaking 0.x
change still passes the handshake. Until 1.0.0, peers must be kept in
step by deployment practice, not by the handshake.

Records that arrive another way (direct client writes, federation,
the firehose) are not version-checked. They are classified against
the local compiled registry: `Tier::from_nsid` returns
`UnknownNsid::NotRegistered` for any `tools.kryphocron.*` NSID the
local build does not know, and a known deprecated NSID gets the
deprecation behavior below.

## No reclassification: deprecate and replace

A lexicon's tier cannot change in place. Different substrate
components are built against the tier compiled into their own
binary. If an NSID could move between tiers, two components built at
different versions would hold tier-typed envelopes for that NSID
that make contradictory privacy promises. The lockfile therefore
makes tier immutable, and there is no reclassification mechanism to
misuse.

To change a lexicon's semantics, introduce a **successor** under a
new NSID with the tier it needs, and deprecate the old one:

```json
"tools.kryphocron.feed.someOldLexicon": {
  "tier": "private",
  "lexicon_version": "1.2.0",
  "deprecated_in": "2.0.0",
  "successor": "tools.kryphocron.feed.someNewLexicon"
},
"tools.kryphocron.feed.someNewLexicon": {
  "tier": "public",
  "lexicon_version": "1.0.0"
}
```

The build turns this into a registry entry with
`DeprecationState::Deprecated { since_version, successor }`. Once set,
`deprecated_in` and `successor` are locked (see
[manifest-and-lockfile.md](manifest-and-lockfile.md)). No lexicon is
deprecated today.

### What deprecation does

These rules are enforced by kryphocron, not by this crate:

- **Existing records** stay readable under their original tier
  indefinitely.
- **New writes** to a deprecated NSID are refused when the
  capability proof is bound to the record
  (`DenialReason::CapabilityDeprecated`). Read capabilities skip this
  check; moderation capabilities always go through it.
- **A grace window.** An operator may set one, which turns a static
  `Deprecated` entry into `DeprecatedWithGrace` at runtime. Inside
  the window, writes proceed and each one is audited, so operators
  can watch clients migrate. The registry itself only ever contains
  `Active` or `Deprecated`.
- **The successor** is an unrelated NSID with its own tier. Nothing
  is derived from the old lexicon.

### Migration and removal

The substrate never migrates records between a deprecated lexicon and
its successor. An operator can leave old records in place (the
default), run their own migration tool (each write is an ordinary
write under the successor's tier), or tombstone the old records.

Lexicons are never removed. The set only grows: a record under a
deprecated NSID stays accessible, and the registry has no way to drop
an NSID.
