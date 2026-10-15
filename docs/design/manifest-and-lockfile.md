# Manifest, lockfile and tier classification

A record's tier (public or private) is substrate metadata. It is not
part of ATProto Lexicon. `proto-blue-lexicon`'s `LexiconDoc` follows
the Lexicon spec strictly, so a `kryphocron` block embedded in the
lexicon JSON would either be dropped or break the spec. The tier
therefore lives in a sidecar file next to the lexicons:
`kryphocron-manifest.json`.

## Why a sidecar

| Option | Verdict |
| --- | --- |
| Encode the tier in the name (`postPublic` / `postPrivate`) | Rejected. It ties wire identity to substrate behavior: renaming would change the tier, and a new tier would be impossible without renaming. |
| Embed metadata in the lexicon JSON | Rejected. It is not valid Lexicon. |
| Extend proto-blue's parser | Rejected. proto-blue is a faithful ATProto implementation and should not carry substrate concerns. |
| A sidecar manifest | Chosen. proto-blue handles the lexicons and this crate handles its own metadata, in the same repository, release and review. |

(The `postPublic` / `postPrivate` names are only labels. The tier
comes from the manifest.)

## `kryphocron-manifest.json`

```json
{
  "manifest_version": "1.0.0",
  "lexicons": {
    "tools.kryphocron.feed.postPrivate": {
      "tier": "private",
      "lexicon_version": "1.0.0"
    },
    "tools.kryphocron.graph.block": {
      "tier": "private",
      "lexicon_version": "1.0.0",
      "audience_exempt": true,
      "exemption_reason": "consumed by BlockOracle outside the normal capability flow"
    },
    "tools.kryphocron.feed.createPostPrivate": {
      "kind": "procedure",
      "lexicon_version": "1.0.0"
    }
  }
}
```

| Field | Meaning |
| --- | --- |
| `manifest_version` | The schema version of the manifest itself. Only `1.0.0` is accepted. A new schema needs explicit build-script support; the build never guesses. |
| `kind` | `record` (the default) or `procedure`. It must match the lexicon's `main` def type. |
| `tier` | `public` or `private`. Required for records, forbidden for procedures. |
| `lexicon_version` | The lexicon's own semver. Required; recorded but not yet used. It is separate from `manifest_version` because the two change on different schedules. |
| `deprecated_in`, `successor` | Optional. See [versioning-and-deprecation.md](versioning-and-deprecation.md). |
| `audience_exempt`, `exemption_reason` | Optional. See the audience rule below. Records only. |

Unknown fields are rejected.

### Kinds

Only records have a tier. A procedure describes an endpoint, not
stored data. A procedure entry therefore carries `kind: "procedure"`
and none of the tier fields (`tier`, `audience_exempt`,
`exemption_reason`). It does not appear in the registry or the
lockfile. The build fails if:

- an entry's `kind` does not match its lexicon's `main` def;
- a record has no `tier`;
- a procedure carries any tier field.

## Tier immutability: `.kryphocron-manifest.lock`

Three manifest fields can never be undone once set:

- **`tier`** never changes for a published NSID. The tier is part of
  the lexicon's identity (see
  [versioning-and-deprecation.md](versioning-and-deprecation.md)).
- **`deprecated_in`**, once set, cannot be cleared or changed.
- **`successor`**, once set, cannot be changed.

A convention would not be enough here. The build enforces all three
against a committed lockfile:

```json
{
  "lock_version": "1.0.0",
  "lexicons": {
    "tools.kryphocron.feed.postPrivate": {
      "tier_locked": "private",
      "deprecated_in_locked": null,
      "successor_locked": null,
      "first_seen_in_lexicon_set_version": "0.1.0"
    }
  }
}
```

On every build:

1. The script checks each record NSID found in both the manifest and
   the lockfile: the tier must equal `tier_locked`, and a locked
   deprecation or successor value must be unchanged.
2. It regenerates the lockfile from the manifest. A record NSID not
   yet locked gets an entry, with `first_seen_in_lexicon_set_version`
   taken from `version.json`.
3. It writes the regenerated file to
   `$OUT_DIR/.kryphocron-manifest.lock.regenerated`, then compares it
   with the committed file, ignoring key order and whitespace. If they
   differ, the build fails and prints the `cp` command that refreshes
   the committed lockfile. Commit the result together with the
   manifest change.

A tier change fails with:

```
lexicon `<nsid>` attempted tier change from `public` to `private`.
Tier is immutable; this is forbidden. To change semantics, introduce
a new NSID as a successor lexicon.
```

### What the lockfile protects

The lockfile works like `Cargo.lock`. To loosen a locked field, a
change has to edit both the manifest and the lockfile, in the same
diff, where a reviewer can see it. That makes the change **visible**;
it does not make it **unforgeable**. The trust boundary is git
history, branch protection and review. A compromised repository or
build pipeline can still produce a malicious change.

## The private-tier audience rule

A private record must say who may read it. Every private-tier record
lexicon must declare an `audienceList` property on its main record,
as a string with `format: "at-uri"`. The value is a reference to a
`tools.kryphocron.policy.audience` record. The build rejects any
private record without one:

```
private-tier lexicon `tools.kryphocron.feed.postPrivate` does not
declare an audience-list reference field (`audienceList:
string<format:at-uri>` referencing a `tools.kryphocron.policy.audience`
record).
```

This catches the most damaging way a manifest and its lexicons can
disagree: a manifest saying "private" over a lexicon shaped like
"public". Catching it does not require a general semantic linter.

- The rule checks that the field exists, under that name, with that
  format. Whether the field is required is up to the lexicon.
- The field is matched by name because other AT-URI fields, such as
  `postPrivate.publicCompanion`, are not audience references.
- The reference is an AT-URI string, not an embedded record. It is a
  live pointer that is read when a record is read; it is not a
  snapshot taken when the record was written.
- `tests/lexicon_invariants.rs` repeats the check independently of
  the build.

### Exemptions

Some private records are substrate infrastructure, not user content.
An entry can set `audience_exempt: true`, but only together with a
non-empty `exemption_reason`.

An exemption is meant only for:

- lexicons that only substrate-class service principals read or
  write; or
- lexicons read under implicit substrate trust by an oracle.

Three entries are exempt today:

- `graph.block`, read by the block oracle;
- `graph.mute`, read by the mute oracle;
- `policy.audience`, the audience mechanism itself. The build
  requires this entry to be marked exempt.

The build checks only that a reason is present. Whether the reason
fits one of the two categories is checked in review. Exemptions are a
narrow escape hatch, not a general way to skip the audience
requirement.

## Build-time authority

The manifest is the authority at build time, not at runtime. A
running substrate trusts only the compiled-in
`KRYPHOCRON_LEXICON_REGISTRY`, and the manifest on disk is never read
at runtime. In practice:

- Editing the manifest in a deployment changes nothing until the
  crate is rebuilt and redeployed.
- The defense against a malicious manifest change is the build
  pipeline: repository write access, review and release signing. The
  substrate guards against its runtime drifting from the compiled
  registry, but it cannot guard against a compromised build.
- Operators with high-assurance needs should sign release builds,
  review every manifest change, and treat a major bump of the
  lexicon set as a significant operational event.
