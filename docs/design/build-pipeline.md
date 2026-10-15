# Build pipeline

`build.rs` turns the lexicon JSON and the sidecar files into
generated Rust types and registry constants. Before producing
anything, it checks that they all agree. Every failure stops the
build with a diagnostic that names the NSIDs involved.

## Steps

```
lexicons/**/*.json ─┐
kryphocron-manifest.json ─┼─► parse ─► stage records ─► proto-blue-codegen ─► generated Rust
.kryphocron-manifest.lock ─┤                                                     │
version.json ─┘                                                                   ▼
                    three-way NSID check ─► manifest kinds ─► lockfile ─► audience rule
                                                                                  │
                                                                                  ▼
                              codegen hash ─► registry.rs, codegen-entry.rs, lexicon_jsons.rs
```

1. **Read the inputs.** The build reads every `*.json` under
   `lexicons/`, the manifest, the lockfile and `version.json`. Each
   input is registered with `cargo:rerun-if-changed`.
2. **Parse the lexicons** as `LexiconDoc`s with `proto-blue-lexicon`.
   Two files with the same NSID fail the build.
3. **Stage the codegen input.** Every lexicon whose `main` def is not
   a procedure is copied into `OUT_DIR/codegen-input/`, keeping its
   relative path. Procedures are left out (see below).
4. **Run codegen.** `OUT_DIR/codegen-output/` is cleared, then
   `proto-blue-codegen --lexicons <staged> --output <out>` runs.
   Clearing first means a lexicon removed since the last build cannot
   leave stale generated files behind.
5. **Run the three-way consistency check** (below).
6. **Check manifest kinds** (see
   [manifest-and-lockfile.md](manifest-and-lockfile.md)).
7. **Validate the lockfile:** tier immutability, monotonic
   deprecation and successor fields, and freshness.
8. **Check the private-tier audience rule.**
9. **Hash the codegen output.** The hash is SHA-256 over each
   generated file's relative path and contents, in path order.
10. **Emit the artifacts** (below).

## Codegen

The lexicon-to-Rust step is `proto-blue-codegen`, which the substrate
already uses for its other ATProto lexicons. One generator keeps the
generated types consistent across all of them. It emits validated
newtypes for formatted strings (`did`, `at-uri`, `datetime` and so
on) and tagged enums for unions, so wire data is type-checked before
it reaches any authorization decision.

`proto-blue-codegen` 0.3.x ships only as a binary. The build script
finds it on `PATH` and runs it as a subprocess. A missing binary
fails the build with the install command:

```bash
cargo install proto-blue-codegen --version '~0.3.1'
```

This applies to anyone building the crate, including downstream
crates that depend on it: the generated code is not committed or
packaged, so every build regenerates it. If `proto-blue-codegen`
later exposes a library API, calling it in-process is a contained
change to one function.

The proto-blue dependencies are pinned to `~0.3.1` (patch updates
only). The registry and the consistency check depend on the
generator's output layout, with one module per NSID at a path derived
from the NSID. A minor release of the generator has to be
re-evaluated before it is adopted. The pin forces that step; Cargo
does not.

### Procedures get no codegen

For a procedure lexicon, `proto-blue-codegen` generates an XRPC
client, which needs `proto-blue-xrpc` and with it an HTTP stack.
kryphocron uses this crate as a build dependency, and no Rust
consumer needs a generated client: a host implements these endpoints
itself. Procedures therefore ship as JSON only. They are part of the
manifest and the consistency check, and they load through
`lexicons()`.

## Three-way consistency

The build compares three NSID sets:

1. the lexicon files under `lexicons/`;
2. the manifest entries;
3. the modules in the codegen output. The NSID comes from each
   module's path.

The lexicon set must equal the manifest set. The codegen set must
equal the set of lexicons handed to codegen, which is every lexicon
except the procedures. A mismatch in any direction fails the build,
and the message lists each NSID under the difference it falls in:

```
NSID set mismatch (three-way consistency)

Lexicons present but missing from manifest:
  tools.kryphocron.feed.quotePrivate

Resolution: update kryphocron-manifest.json to match the
lexicons/ directory, then regenerate.
```

The check compares sets only. It does not check that a lexicon's
schema matches its tier. A record classified `private` with a public
shape passes it. The private-tier audience rule closes the most
important form of that gap; anything else is left to review.

The NSID sets are computed by the build script itself, not collected
at runtime through linker tricks such as `inventory` or `linkme`.
Such tricks do not work on WASM, and dead-code elimination can drop
their registrations silently. The trade-off is that a bug in the
build script that reads NSIDs from the codegen output wrongly could
agree with itself. The script is short, and the version pin limits
how far the output layout can drift.

## Generated artifacts

All three are written to `OUT_DIR` and pulled in with `include!`:

- **`codegen-entry.rs`** mounts the codegen output at `crate::tools`
  with a `#[path]` module, so the generated
  `crate::tools::kryphocron::…` paths resolve.
- **`registry.rs`** defines `KRYPHOCRON_CODEGEN_HASH`,
  `KRYPHOCRON_LEXICON_REGISTRY` (one entry per record NSID, sorted)
  and `impl Tier { pub fn from_nsid }`. `from_nsid` is a `match`
  with one arm per registry entry, so adding a lexicon adds an arm.
  Procedures are not in the registry.
- **`lexicon_jsons.rs`** pairs each NSID, sorted, with the verbatim
  JSON. On first call, `lexicons()` builds a `Lexicons` collection
  from it and caches the result in a `OnceLock`. Malformed embedded
  JSON panics, because that would mean the crate itself is broken.

## The codegen hash

The generated code is never committed. It lives in `OUT_DIR` and is
rebuilt from the lexicons on every build. A hand edit to a generated
file therefore cannot survive a rebuild, because regeneration
overwrites it.

`KRYPHOCRON_CODEGEN_HASH` publishes a digest of what was generated,
so two builds can be compared. The build does not check it against a
committed expected value. A committed hash file would be the stricter
design, but it would need updating on every lexicon change and every
generator bump. With uncommitted output, it would guard against
little. To extend a generated type, add trait impls in a separate
module; never edit the generated source.

## Failure modes

| Error | Cause |
| --- | --- |
| `CodegenBinaryMissing` | `proto-blue-codegen` is not on `PATH`. |
| `CodegenFailed` | The generator exited non-zero. |
| `LexiconParse`, `DuplicateLexicon` | A lexicon file does not parse, or two share an NSID. |
| `ManifestSchemaVersion`, `LockfileSchemaVersion` | An unrecognized `manifest_version` or `lock_version`. Only `1.0.0` is accepted, and the build never guesses at another schema. |
| `NsidSetMismatch` | The three-way check failed. |
| `KindMismatch`, `RecordMissingTier`, `ProcedureCarriesTierFields` | A manifest `kind` disagrees with the lexicon, a record has no tier, or a procedure has tier fields. |
| `TierImmutableViolated` | A record's tier changed against the lockfile. |
| `DeprecationMonotonicityViolated`, `SuccessorMonotonicityViolated` | A locked `deprecated_in` or `successor` value changed or was cleared. |
| `StaleLockfile` | The committed lockfile differs from the one regenerated from the manifest. |
| `PrivateLexiconMissingAudienceRef` | A non-exempt private record has no `audienceList` AT-URI field. |
| `MissingExemptionReason`, `AudienceMechanismMustBeExempt` | An exemption without a reason, or `policy.audience` not marked exempt. |
| `UnknownTier` | A tier other than `public` or `private`. |

The tests in `tests/lexicon_invariants.rs` pin the resulting state:
the registry contents, `from_nsid`, the audience rule, and the
procedures staying out of the registry. They do not drive the failure
paths, because doing so needs a deliberately broken manifest
committed to the tree. Those paths are checked by breaking a scratch
copy of the repository and building it.
