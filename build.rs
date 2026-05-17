//! Build script for the `kryphocron-lexicons` companion crate.
//!
//! Pipeline shape (per §5.2 / §5.3 / §5.4):
//!
//! 1. Discover the `proto-blue-codegen` binary on `PATH` via the
//!    `which` crate. The §5.2 primary library-API integration path
//!    requires `proto-blue-codegen` to expose `Generator` and
//!    `load_lexicons` from a `lib.rs`; v0.3.x ships binary-only,
//!    so the build uses the subprocess fallback.
//! 2. Invoke the binary as a subprocess against `lexicons/` and
//!    write its output to `OUT_DIR/codegen-output/`.
//! 3. Parse `kryphocron-manifest.json` and `.kryphocron-manifest.lock`
//!    plus `version.json`.
//! 4. Three-way consistency check (§5.3): the NSID sets from
//!    `lexicons/`, the manifest, and the codegen output must
//!    match. Mismatch in any direction is a build failure with a
//!    diagnostic message naming the offending NSID(s).
//! 5. Lockfile validation (§5.4): `tier` immutable, `deprecated_in`
//!    monotonic, `successor` monotonic. New lexicons added to the
//!    lockfile inherit `first_seen_in_lexicon_set_version` from
//!    `version.json`. Stale committed lockfile fails with a
//!    regenerate-and-commit instruction.
//! 6. Private-tier structural validation (§5.4): every private-tier
//!    lexicon must declare an `audienceList: ref<...policy.audience>`
//!    field unless the manifest entry carries `audience_exempt: true`
//!    with an `exemption_reason`. Exemption is only valid for
//!    substrate-class capabilities or implicit-substrate-trust oracle
//!    consumers.
//! 7. Hash-based hand-edit rejection (§5.3): the build emits a
//!    SHA-256 digest of the concatenated codegen output. The
//!    runtime crate exposes it as a constant; hand-editing any
//!    generated file produces a different hash, which CI compares
//!    against the committed value.
//! 8. Emit `OUT_DIR/codegen-entry.rs` (path-redirected entrypoint
//!    that mounts the codegen output at `crate::tools`) and
//!    `OUT_DIR/registry.rs` (the `KRYPHOCRON_LEXICON_REGISTRY`
//!    constant plus the `impl Tier { pub const fn from_nsid }`
//!    generated from the registry).
//!
//! Operators rebuilding kryphocron-lexicons after lexicon JSON,
//! manifest, or lockfile changes will see the relevant failure mode
//! surface as a compile-time error.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use proto_blue_lexicon::LexiconDoc;
use serde::Deserialize;
use sha2::Digest;

// ============================================================
// Top-level entrypoint.
// ============================================================

fn main() {
    if let Err(err) = run() {
        eprintln!("kryphocron-lexicons build failure:\n\n{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), BuildError> {
    let manifest_dir = PathBuf::from(env_var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env_var("OUT_DIR")?);

    // Force re-run on any input file change.
    rerun_if_inputs_changed(&manifest_dir)?;

    // ---- Read inputs ----
    let lexicons_dir = manifest_dir.join("lexicons");
    let manifest_path = manifest_dir.join("kryphocron-manifest.json");
    let lock_path = manifest_dir.join(".kryphocron-manifest.lock");
    let version_path = manifest_dir.join("version.json");

    let manifest = read_manifest(&manifest_path)?;
    let lockfile = read_lockfile(&lock_path)?;
    let version_doc = read_version(&version_path)?;

    let lexicon_files = discover_lexicon_files(&lexicons_dir)?;
    let parsed_lexicons = parse_lexicons(&lexicon_files)?;

    // ---- Run codegen ----
    let codegen_out = out_dir.join("codegen-output");
    fs::create_dir_all(&codegen_out)
        .map_err(|e| BuildError::Io(format!("create codegen-output dir: {e}")))?;
    run_codegen(&lexicons_dir, &codegen_out)?;
    let codegen_nsids = discover_codegen_nsids(&codegen_out)?;

    // ---- Three-way consistency (§5.3) ----
    let lexicon_nsids: BTreeSet<String> =
        parsed_lexicons.keys().cloned().collect();
    let manifest_nsids: BTreeSet<String> =
        manifest.lexicons.keys().cloned().collect();

    three_way_consistency_check(&lexicon_nsids, &manifest_nsids, &codegen_nsids)?;

    // ---- Lockfile validation (§5.4) ----
    let updated_lockfile = validate_lockfile(&manifest, &lockfile, &version_doc)?;
    write_lockfile_freshness_artifact(&out_dir, &lock_path, &updated_lockfile)?;

    // ---- Private-tier structural validation (§5.4) ----
    validate_private_tier_audience_refs(&manifest, &parsed_lexicons)?;

    // ---- Hash codegen output (§5.3) ----
    let codegen_hash = hash_codegen_output(&codegen_out)?;

    // ---- Emit generated artifacts ----
    write_codegen_entry(&out_dir, &codegen_out)?;
    write_registry(&out_dir, &manifest, &codegen_hash)?;

    Ok(())
}

fn env_var(name: &str) -> Result<String, BuildError> {
    std::env::var(name).map_err(|_| BuildError::Env(name.to_string()))
}

fn rerun_if_inputs_changed(manifest_dir: &Path) -> Result<(), BuildError> {
    for rel in [
        "kryphocron-manifest.json",
        ".kryphocron-manifest.lock",
        "version.json",
        "build.rs",
    ] {
        println!("cargo:rerun-if-changed={}", manifest_dir.join(rel).display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("lexicons").display()
    );
    // Recursively register every lexicon file.
    visit_lexicon_files(&manifest_dir.join("lexicons"), &mut |p| {
        println!("cargo:rerun-if-changed={}", p.display());
        Ok(())
    })?;
    Ok(())
}

// ============================================================
// File discovery.
// ============================================================

fn discover_lexicon_files(dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let mut out = Vec::new();
    visit_lexicon_files(dir, &mut |p| {
        out.push(p.to_path_buf());
        Ok(())
    })?;
    out.sort();
    Ok(out)
}

fn visit_lexicon_files<F>(dir: &Path, f: &mut F) -> Result<(), BuildError>
where
    F: FnMut(&Path) -> Result<(), BuildError>,
{
    if !dir.exists() {
        return Err(BuildError::Io(format!(
            "lexicons directory not found at {}",
            dir.display()
        )));
    }
    let entries = fs::read_dir(dir).map_err(|e| {
        BuildError::Io(format!("read_dir {}: {e}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| BuildError::Io(format!("read_dir entry: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            visit_lexicon_files(&path, f)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            f(&path)?;
        }
    }
    Ok(())
}

// ============================================================
// Manifest / lockfile / version document.
// ============================================================

#[derive(Debug, Deserialize)]
struct Manifest {
    manifest_version: String,
    lexicons: BTreeMap<String, ManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntry {
    tier: String,
    // §5.4 commits per-lexicon version on every entry; we
    // deserialize it to enforce its presence as a structural
    // schema check (deserialize fails if absent), but v0.1 does
    // not yet consume the value.
    #[allow(dead_code)]
    lexicon_version: String,
    #[serde(default)]
    deprecated_in: Option<String>,
    #[serde(default)]
    successor: Option<String>,
    #[serde(default)]
    audience_exempt: bool,
    #[serde(default)]
    exemption_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Lockfile {
    #[allow(dead_code)]
    lock_version: String,
    lexicons: BTreeMap<String, LockEntry>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LockEntry {
    tier_locked: String,
    deprecated_in_locked: Option<String>,
    successor_locked: Option<String>,
    first_seen_in_lexicon_set_version: String,
}

#[derive(Debug, Deserialize)]
struct VersionDoc {
    version: String,
    #[allow(dead_code)]
    #[serde(default)]
    compatibility: serde_json::Value,
}

fn read_manifest(p: &Path) -> Result<Manifest, BuildError> {
    let raw = fs::read_to_string(p)
        .map_err(|e| BuildError::Io(format!("read {}: {e}", p.display())))?;
    let m: Manifest = serde_json::from_str(&raw)
        .map_err(|e| BuildError::ManifestParse(format!("{e}")))?;
    if m.manifest_version != "1.0.0" {
        return Err(BuildError::ManifestSchemaVersion(m.manifest_version));
    }
    Ok(m)
}

fn read_lockfile(p: &Path) -> Result<Option<Lockfile>, BuildError> {
    if !p.exists() {
        // §5.4 special-cases the first build: empty lockfile means
        // "create on first build". Return None; downstream creates.
        return Ok(None);
    }
    let raw = fs::read_to_string(p)
        .map_err(|e| BuildError::Io(format!("read {}: {e}", p.display())))?;
    let lf: Lockfile = serde_json::from_str(&raw)
        .map_err(|e| BuildError::LockfileParse(format!("{e}")))?;
    if lf.lock_version != "1.0.0" {
        return Err(BuildError::LockfileSchemaVersion(lf.lock_version));
    }
    Ok(Some(lf))
}

fn read_version(p: &Path) -> Result<VersionDoc, BuildError> {
    let raw = fs::read_to_string(p)
        .map_err(|e| BuildError::Io(format!("read {}: {e}", p.display())))?;
    serde_json::from_str(&raw).map_err(|e| BuildError::VersionParse(format!("{e}")))
}

// ============================================================
// Lexicon parsing (for private-tier structural validation).
// ============================================================

fn parse_lexicons(paths: &[PathBuf]) -> Result<BTreeMap<String, LexiconDoc>, BuildError> {
    let mut out = BTreeMap::new();
    for path in paths {
        let raw = fs::read_to_string(path)
            .map_err(|e| BuildError::Io(format!("read {}: {e}", path.display())))?;
        let doc: LexiconDoc = serde_json::from_str(&raw)
            .map_err(|e| BuildError::LexiconParse {
                path: path.display().to_string(),
                detail: format!("{e}"),
            })?;
        let nsid = doc.id.clone();
        if out.insert(nsid.clone(), doc).is_some() {
            return Err(BuildError::DuplicateLexicon(nsid));
        }
    }
    Ok(out)
}

// ============================================================
// Codegen subprocess invocation (§5.2 fallback path).
// ============================================================

fn run_codegen(lexicons_dir: &Path, out_dir: &Path) -> Result<(), BuildError> {
    let codegen_binary = which::which("proto-blue-codegen").map_err(|_| {
        BuildError::CodegenBinaryMissing
    })?;

    let status = Command::new(&codegen_binary)
        .arg("--lexicons")
        .arg(lexicons_dir)
        .arg("--output")
        .arg(out_dir)
        .status()
        .map_err(|e| BuildError::CodegenSpawn(format!("{e}")))?;

    if !status.success() {
        return Err(BuildError::CodegenFailed(
            status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}

fn discover_codegen_nsids(codegen_out: &Path) -> Result<BTreeSet<String>, BuildError> {
    let mut out = BTreeSet::new();
    visit_codegen_files(codegen_out, &mut |p, relpath| {
        // Files at <out>/tools/kryphocron/.../<lexicon>.rs map back
        // to NSID `tools.kryphocron.....<lexicon>`. Skip the
        // synthesized mod.rs files.
        let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if fname == "mod.rs" {
            return Ok(());
        }
        // relpath looks like `tools/kryphocron/feed/postPrivate.rs`
        let nsid_path = relpath
            .trim_end_matches(".rs")
            .replace(std::path::MAIN_SEPARATOR, ".");
        if nsid_path.starts_with("tools.kryphocron.") {
            out.insert(nsid_path);
        }
        Ok(())
    })?;
    Ok(out)
}

fn visit_codegen_files<F>(dir: &Path, f: &mut F) -> Result<(), BuildError>
where
    F: FnMut(&Path, &str) -> Result<(), BuildError>,
{
    fn inner<F>(root: &Path, dir: &Path, f: &mut F) -> Result<(), BuildError>
    where
        F: FnMut(&Path, &str) -> Result<(), BuildError>,
    {
        let entries = fs::read_dir(dir).map_err(|e| {
            BuildError::Io(format!("read_dir {}: {e}", dir.display()))
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| BuildError::Io(format!("read_dir entry: {e}")))?;
            let path = entry.path();
            if path.is_dir() {
                inner(root, &path, f)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path.strip_prefix(root).unwrap();
                let rel_str = rel.to_string_lossy().to_string();
                f(&path, &rel_str)?;
            }
        }
        Ok(())
    }
    inner(dir, dir, f)
}

// ============================================================
// Three-way consistency (§5.3).
// ============================================================

fn three_way_consistency_check(
    lexicon: &BTreeSet<String>,
    manifest: &BTreeSet<String>,
    codegen: &BTreeSet<String>,
) -> Result<(), BuildError> {
    let lexicon_minus_manifest: Vec<String> =
        lexicon.difference(manifest).cloned().collect();
    let manifest_minus_lexicon: Vec<String> =
        manifest.difference(lexicon).cloned().collect();
    let lexicon_minus_codegen: Vec<String> =
        lexicon.difference(codegen).cloned().collect();
    let codegen_minus_lexicon: Vec<String> =
        codegen.difference(lexicon).cloned().collect();

    if lexicon_minus_manifest.is_empty()
        && manifest_minus_lexicon.is_empty()
        && lexicon_minus_codegen.is_empty()
        && codegen_minus_lexicon.is_empty()
    {
        return Ok(());
    }

    Err(BuildError::NsidSetMismatch {
        lexicon_minus_manifest,
        manifest_minus_lexicon,
        lexicon_minus_codegen,
        codegen_minus_lexicon,
    })
}

// ============================================================
// Lockfile validation (§5.4).
// ============================================================

fn validate_lockfile(
    manifest: &Manifest,
    lockfile: &Option<Lockfile>,
    version: &VersionDoc,
) -> Result<Lockfile, BuildError> {
    let prior = lockfile.clone().unwrap_or_else(|| Lockfile {
        lock_version: "1.0.0".to_string(),
        lexicons: BTreeMap::new(),
    });

    let mut updated = Lockfile {
        lock_version: "1.0.0".to_string(),
        lexicons: BTreeMap::new(),
    };

    for (nsid, entry) in &manifest.lexicons {
        let new_lock = LockEntry {
            tier_locked: entry.tier.clone(),
            deprecated_in_locked: entry.deprecated_in.clone(),
            successor_locked: entry.successor.clone(),
            first_seen_in_lexicon_set_version: prior
                .lexicons
                .get(nsid)
                .map_or_else(
                    || version.version.clone(),
                    |e| e.first_seen_in_lexicon_set_version.clone(),
                ),
        };

        if let Some(prior_entry) = prior.lexicons.get(nsid) {
            // §5.4: tier is immutable.
            if prior_entry.tier_locked != new_lock.tier_locked {
                return Err(BuildError::TierImmutableViolated {
                    nsid: nsid.clone(),
                    locked: prior_entry.tier_locked.clone(),
                    current: new_lock.tier_locked.clone(),
                });
            }
            // §5.4: deprecated_in is monotonic — once set, cannot
            // be cleared or changed.
            if prior_entry.deprecated_in_locked.is_some()
                && prior_entry.deprecated_in_locked != new_lock.deprecated_in_locked
            {
                return Err(BuildError::DeprecationMonotonicityViolated {
                    nsid: nsid.clone(),
                    locked: prior_entry.deprecated_in_locked.clone(),
                    current: new_lock.deprecated_in_locked.clone(),
                });
            }
            // §5.4: successor is monotonic — once set, cannot change.
            if prior_entry.successor_locked.is_some()
                && prior_entry.successor_locked != new_lock.successor_locked
            {
                return Err(BuildError::SuccessorMonotonicityViolated {
                    nsid: nsid.clone(),
                    locked: prior_entry.successor_locked.clone(),
                    current: new_lock.successor_locked.clone(),
                });
            }
        }

        updated.lexicons.insert(nsid.clone(), new_lock);
    }

    Ok(updated)
}

fn write_lockfile_freshness_artifact(
    out_dir: &Path,
    committed_lock_path: &Path,
    regenerated: &Lockfile,
) -> Result<(), BuildError> {
    let regenerated_text = serde_json::to_string_pretty(&LockfileSerialize::from(regenerated))
        .map_err(|e| BuildError::Io(format!("serialize regenerated lockfile: {e}")))?;
    let regenerated_text = format!("{regenerated_text}\n");

    // Write to OUT_DIR so operators can inspect during a stale-lock
    // failure: `cp $OUT_DIR/.kryphocron-manifest.lock.regenerated
    // .kryphocron-manifest.lock`.
    let staging = out_dir.join(".kryphocron-manifest.lock.regenerated");
    fs::write(&staging, &regenerated_text)
        .map_err(|e| BuildError::Io(format!("write {}: {e}", staging.display())))?;

    if committed_lock_path.exists() {
        let committed = fs::read_to_string(committed_lock_path).map_err(|e| {
            BuildError::Io(format!("read {}: {e}", committed_lock_path.display()))
        })?;
        if normalize_json_text(&committed) != normalize_json_text(&regenerated_text) {
            return Err(BuildError::StaleLockfile {
                staging: staging.display().to_string(),
                committed: committed_lock_path.display().to_string(),
            });
        }
    }

    Ok(())
}

/// Sort and re-serialize JSON to ignore whitespace and key-order
/// differences when comparing committed vs regenerated lockfile.
fn normalize_json_text(s: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(s) else {
        return s.to_string();
    };
    let canonical = canonicalize_json(v);
    serde_json::to_string(&canonical).unwrap_or_default()
}

/// Recursively rebuild a [`serde_json::Value`] with object keys in
/// sorted order. With `serde_json/preserve_order` enabled the
/// `Map` type is `IndexMap`, which preserves insertion order; the
/// comparison the lockfile-freshness check wants is structural, so
/// we explicitly canonicalize.
fn canonicalize_json(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => {
            let sorted: BTreeMap<String, serde_json::Value> = m
                .into_iter()
                .map(|(k, v)| (k, canonicalize_json(v)))
                .collect();
            let mut out = serde_json::Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.into_iter().map(canonicalize_json).collect())
        }
        other => other,
    }
}

// Local serialize wrapper that mirrors LockEntry's field order
// without requiring a Serialize derive on the deserialize-side
// types. (Avoids exposing serde::Serialize publicly.)
#[derive(serde::Serialize)]
struct LockfileSerialize<'a> {
    lock_version: &'a str,
    lexicons: BTreeMap<&'a str, LockEntrySerialize<'a>>,
}

#[derive(serde::Serialize)]
struct LockEntrySerialize<'a> {
    tier_locked: &'a str,
    deprecated_in_locked: Option<&'a str>,
    successor_locked: Option<&'a str>,
    first_seen_in_lexicon_set_version: &'a str,
}

impl<'a> From<&'a Lockfile> for LockfileSerialize<'a> {
    fn from(lf: &'a Lockfile) -> Self {
        LockfileSerialize {
            lock_version: &lf.lock_version,
            lexicons: lf
                .lexicons
                .iter()
                .map(|(k, v)| (k.as_str(), LockEntrySerialize::from(v)))
                .collect(),
        }
    }
}

impl<'a> From<&'a LockEntry> for LockEntrySerialize<'a> {
    fn from(e: &'a LockEntry) -> Self {
        LockEntrySerialize {
            tier_locked: &e.tier_locked,
            deprecated_in_locked: e.deprecated_in_locked.as_deref(),
            successor_locked: e.successor_locked.as_deref(),
            first_seen_in_lexicon_set_version: &e.first_seen_in_lexicon_set_version,
        }
    }
}

// ============================================================
// Private-tier structural validation (§5.4).
// ============================================================

fn validate_private_tier_audience_refs(
    manifest: &Manifest,
    parsed_lexicons: &BTreeMap<String, LexiconDoc>,
) -> Result<(), BuildError> {
    const AUDIENCE_REF: &str = "tools.kryphocron.policy.audience";

    for (nsid, entry) in &manifest.lexicons {
        if entry.tier != "private" {
            continue;
        }

        if entry.audience_exempt {
            // §5.4 commits: exemption requires a documented reason.
            if entry.exemption_reason.as_deref().unwrap_or("").is_empty() {
                return Err(BuildError::MissingExemptionReason(nsid.clone()));
            }
            continue;
        }

        // policy.audience itself is the audience mechanism; can't
        // be enforced to reference itself. Surface this as a
        // misconfiguration explicitly (operator must add
        // audience_exempt: true with reason).
        if nsid == AUDIENCE_REF {
            return Err(BuildError::AudienceMechanismMustBeExempt {
                nsid: nsid.clone(),
            });
        }

        let doc = parsed_lexicons.get(nsid).ok_or_else(|| {
            BuildError::ManifestParse(format!("private-tier NSID `{nsid}` has no lexicon JSON"))
        })?;

        if !lexicon_declares_audience_ref(doc, AUDIENCE_REF) {
            return Err(BuildError::PrivateLexiconMissingAudienceRef {
                nsid: nsid.clone(),
                expected_ref: AUDIENCE_REF.to_string(),
            });
        }
    }

    Ok(())
}

fn lexicon_declares_audience_ref(doc: &LexiconDoc, target: &str) -> bool {
    use proto_blue_lexicon::types::{LexObject, LexUserType};

    let Some(main) = doc.defs.get("main") else {
        return false;
    };

    let object: &LexObject = match main {
        LexUserType::Record(r) => &r.record,
        LexUserType::Object(o) => o,
        _ => return false,
    };

    object.properties.values().any(|prop| match prop {
        LexUserType::Ref(r) => r.ref_target == target,
        _ => false,
    })
}

// ============================================================
// Hash-based hand-edit rejection (§5.3).
// ============================================================

fn hash_codegen_output(dir: &Path) -> Result<String, BuildError> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    visit_codegen_files(dir, &mut |p, rel| {
        let mut buf = Vec::new();
        fs::File::open(p)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| BuildError::Io(format!("read {}: {e}", p.display())))?;
        entries.push((rel.to_string(), buf));
        Ok(())
    })?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = sha2::Sha256::new();
    for (rel, bytes) in entries {
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(&bytes);
        hasher.update(b"\0");
    }
    let digest = hasher.finalize();
    Ok(hex(&digest))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

// ============================================================
// Code emission.
// ============================================================

fn write_codegen_entry(out_dir: &Path, codegen_out: &Path) -> Result<(), BuildError> {
    // The path-redirected entrypoint mounts codegen output at
    // `crate::tools::*` so generated `crate::tools::kryphocron::...`
    // references resolve correctly.
    let tools_mod = codegen_out.join("tools").join("mod.rs");
    let mut s = String::new();
    s.push_str("// Generated by kryphocron-lexicons/build.rs. Do not edit.\n\n");
    s.push_str("/// Codegen output — proto-blue-codegen module tree, mounted at `crate::tools`.\n");
    // The generated code does not carry rustdoc comments; suppress
    // the crate-level `missing_docs` lint for this subtree. Same
    // for clippy lints that fire on generated code shapes.
    s.push_str("#[allow(missing_docs)]\n");
    s.push_str("#[allow(clippy::all)]\n");
    s.push_str(&format!(
        "#[path = {:?}]\npub mod tools;\n",
        tools_mod.display().to_string()
    ));

    let dest = out_dir.join("codegen-entry.rs");
    fs::write(&dest, s).map_err(|e| BuildError::Io(format!("write {}: {e}", dest.display())))?;
    Ok(())
}

fn write_registry(
    out_dir: &Path,
    manifest: &Manifest,
    codegen_hash: &str,
) -> Result<(), BuildError> {
    let mut s = String::new();
    s.push_str("// Generated by kryphocron-lexicons/build.rs. Do not edit.\n\n");

    // ---- KRYPHOCRON_CODEGEN_HASH constant ----
    s.push_str("/// SHA-256 digest of the concatenated proto-blue-codegen output.\n");
    s.push_str("///\n");
    s.push_str("/// Edit any file under the generated `tools` tree and the\n");
    s.push_str("/// rebuilt hash differs. §5.3 hand-edit rejection: operators\n");
    s.push_str("/// who genuinely need to extend generated types should do so via\n");
    s.push_str("/// trait impls in separate modules.\n");
    s.push_str(&format!(
        "pub const KRYPHOCRON_CODEGEN_HASH: &str = {codegen_hash:?};\n\n"
    ));

    // ---- KRYPHOCRON_LEXICON_REGISTRY constant ----
    s.push_str("/// Authoritative registry of v1 lexicons (§5.3).\n");
    s.push_str("///\n");
    s.push_str("/// Generated from `kryphocron-manifest.json` at build time. The\n");
    s.push_str("/// substrate's runtime trust anchor is this compiled-in constant\n");
    s.push_str("/// (§5.4 build-time-authoritative discipline); the on-disk\n");
    s.push_str("/// manifest is consulted at build time only.\n");
    s.push_str(
        "pub const KRYPHOCRON_LEXICON_REGISTRY: &[crate::LexiconRegistryEntry] = &[\n",
    );

    let mut entries: Vec<(&String, &ManifestEntry)> = manifest.lexicons.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (nsid, entry) in &entries {
        let tier_path = match entry.tier.as_str() {
            "public" => "crate::Tier::Public",
            "private" => "crate::Tier::Private",
            other => {
                // Build script already rejected unknown tiers via
                // the lockfile validation path; defense in depth.
                return Err(BuildError::UnknownTier(other.to_string()));
            }
        };
        let deprecation = match (&entry.deprecated_in, &entry.successor) {
            (None, _) => "crate::DeprecationState::Active".to_string(),
            (Some(ver), succ) => {
                let semver = parse_semver(ver).ok_or_else(|| {
                    BuildError::ManifestParse(format!(
                        "deprecated_in `{ver}` on `{nsid}` is not a valid semver"
                    ))
                })?;
                let succ_lit = succ
                    .as_deref()
                    .map_or("None".to_string(), |s| format!("Some({s:?})"));
                format!(
                    "crate::DeprecationState::Deprecated {{ since_version: \
                     crate::SemVer::new({}, {}, {}), successor: {} }}",
                    semver.0, semver.1, semver.2, succ_lit
                )
            }
        };

        s.push_str(&format!(
            "    crate::LexiconRegistryEntry {{ nsid: {nsid:?}, tier: {tier_path}, deprecation: {deprecation} }},\n"
        ));
    }
    s.push_str("];\n\n");

    // ---- impl Tier { pub const fn from_nsid } ----
    s.push_str("impl crate::Tier {\n");
    s.push_str("    /// Map an NSID to its tier classification via the closed-\n");
    s.push_str("    /// namespace registry (§4.1, §5.3). `const fn`: the match is\n");
    s.push_str("    /// generated from the registry at build time.\n");
    s.push_str("    ///\n");
    s.push_str("    /// # Errors\n");
    s.push_str("    ///\n");
    s.push_str("    /// Returns [`crate::UnknownNsid::NotRegistered`] for any NSID\n");
    s.push_str("    /// not present in `KRYPHOCRON_LEXICON_REGISTRY`.\n");
    s.push_str(
        "    pub fn from_nsid(nsid: &proto_blue_syntax::Nsid) -> Result<crate::Tier, crate::UnknownNsid> {\n",
    );
    s.push_str("        match nsid.as_str() {\n");
    for (nsid, entry) in &entries {
        let tier_path = match entry.tier.as_str() {
            "public" => "crate::Tier::Public",
            "private" => "crate::Tier::Private",
            _ => unreachable!(),
        };
        s.push_str(&format!("            {nsid:?} => Ok({tier_path}),\n"));
    }
    s.push_str(
        "            _ => Err(crate::UnknownNsid::NotRegistered(nsid.clone())),\n",
    );
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("}\n");

    let dest = out_dir.join("registry.rs");
    fs::write(&dest, s)
        .map_err(|e| BuildError::Io(format!("write {}: {e}", dest.display())))?;
    Ok(())
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

// ============================================================
// Errors. (Build-script-internal; not part of the crate's public
// surface.)
// ============================================================

#[derive(Debug)]
enum BuildError {
    Env(String),
    Io(String),
    CodegenBinaryMissing,
    CodegenSpawn(String),
    CodegenFailed(i32),
    ManifestParse(String),
    ManifestSchemaVersion(String),
    LockfileParse(String),
    LockfileSchemaVersion(String),
    VersionParse(String),
    LexiconParse {
        path: String,
        detail: String,
    },
    DuplicateLexicon(String),
    NsidSetMismatch {
        lexicon_minus_manifest: Vec<String>,
        manifest_minus_lexicon: Vec<String>,
        lexicon_minus_codegen: Vec<String>,
        codegen_minus_lexicon: Vec<String>,
    },
    TierImmutableViolated {
        nsid: String,
        locked: String,
        current: String,
    },
    DeprecationMonotonicityViolated {
        nsid: String,
        locked: Option<String>,
        current: Option<String>,
    },
    SuccessorMonotonicityViolated {
        nsid: String,
        locked: Option<String>,
        current: Option<String>,
    },
    StaleLockfile {
        staging: String,
        committed: String,
    },
    PrivateLexiconMissingAudienceRef {
        nsid: String,
        expected_ref: String,
    },
    MissingExemptionReason(String),
    AudienceMechanismMustBeExempt {
        nsid: String,
    },
    UnknownTier(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Env(v) => write!(f, "missing environment variable `{v}`"),
            BuildError::Io(d) => write!(f, "io: {d}"),
            BuildError::CodegenBinaryMissing => write!(
                f,
                "proto-blue-codegen binary not found on PATH.\n\
                 \n\
                 §5.2's subprocess-fallback integration path requires the\n\
                 codegen binary be installed. Run:\n\
                 \n\
                 \x20   cargo install proto-blue-codegen --version ~0.3.1\n\
                 \n\
                 then rebuild kryphocron-lexicons."
            ),
            BuildError::CodegenSpawn(d) => write!(f, "could not spawn proto-blue-codegen: {d}"),
            BuildError::CodegenFailed(code) => {
                write!(f, "proto-blue-codegen exited with status {code}")
            }
            BuildError::ManifestParse(d) => {
                write!(f, "kryphocron-manifest.json parse error: {d}")
            }
            BuildError::ManifestSchemaVersion(v) => write!(
                f,
                "manifest_version `{v}` is not recognized.\n\
                 v0.1 supports manifest_version = \"1.0.0\" only;\n\
                 unrecognized schemas are rejected explicitly per §5.4."
            ),
            BuildError::LockfileParse(d) => {
                write!(f, ".kryphocron-manifest.lock parse error: {d}")
            }
            BuildError::LockfileSchemaVersion(v) => {
                write!(f, "lock_version `{v}` is not recognized (v0.1 supports 1.0.0)")
            }
            BuildError::VersionParse(d) => write!(f, "version.json parse error: {d}"),
            BuildError::LexiconParse { path, detail } => {
                write!(f, "lexicon parse error at {path}: {detail}")
            }
            BuildError::DuplicateLexicon(nsid) => {
                write!(f, "duplicate lexicon NSID `{nsid}` in lexicons directory")
            }
            BuildError::NsidSetMismatch {
                lexicon_minus_manifest,
                manifest_minus_lexicon,
                lexicon_minus_codegen,
                codegen_minus_lexicon,
            } => {
                writeln!(f, "NSID set mismatch (§5.3 three-way consistency)\n")?;
                if !lexicon_minus_manifest.is_empty() {
                    writeln!(f, "Lexicons present but missing from manifest:")?;
                    for n in lexicon_minus_manifest {
                        writeln!(f, "  {n}")?;
                    }
                    writeln!(f)?;
                }
                if !manifest_minus_lexicon.is_empty() {
                    writeln!(f, "Manifest entries with no corresponding lexicon:")?;
                    for n in manifest_minus_lexicon {
                        writeln!(f, "  {n}")?;
                    }
                    writeln!(f)?;
                }
                if !lexicon_minus_codegen.is_empty() {
                    writeln!(f, "Lexicons present but missing from codegen output:")?;
                    for n in lexicon_minus_codegen {
                        writeln!(f, "  {n}")?;
                    }
                    writeln!(f)?;
                }
                if !codegen_minus_lexicon.is_empty() {
                    writeln!(f, "Codegen output with no corresponding lexicon:")?;
                    for n in codegen_minus_lexicon {
                        writeln!(f, "  {n}")?;
                    }
                    writeln!(f)?;
                }
                write!(
                    f,
                    "Resolution: update kryphocron-manifest.json to match the\n\
                     lexicons/ directory, then regenerate."
                )
            }
            BuildError::TierImmutableViolated { nsid, locked, current } => write!(
                f,
                "lexicon `{nsid}` attempted tier change from `{locked}` to `{current}`. \
                 Tier is immutable; this is forbidden. To change semantics, introduce \
                 a new NSID as a successor lexicon (§5.5 deprecate-and-replace)."
            ),
            BuildError::DeprecationMonotonicityViolated { nsid, locked, current } => write!(
                f,
                "lexicon `{nsid}` attempted deprecation reversal (locked={locked:?}, current={current:?}). \
                 deprecated_in is monotonic per §5.4; once set, it cannot be removed or reversed."
            ),
            BuildError::SuccessorMonotonicityViolated { nsid, locked, current } => write!(
                f,
                "lexicon `{nsid}` attempted successor change (locked={locked:?}, current={current:?}). \
                 Once set, successor is immutable per §5.4."
            ),
            BuildError::StaleLockfile { staging, committed } => write!(
                f,
                "lockfile is stale; regenerate and commit.\n\
                 \n\
                 The build script regenerated the lockfile based on the current\n\
                 manifest. The regenerated file does not match the committed one.\n\
                 \n\
                 To fix: copy the regenerated file over the committed one:\n\
                 \n\
                 \x20   cp {staging} {committed}\n\
                 \n\
                 and commit the change alongside the manifest update."
            ),
            BuildError::PrivateLexiconMissingAudienceRef { nsid, expected_ref } => write!(
                f,
                "private-tier lexicon `{nsid}` does not declare an audience-list reference field \
                 (`audienceList: ref<{expected_ref}>`).\n\
                 \n\
                 Private-tier lexicons must declare audience gating per §5.4 to be structurally \
                 consistent with their tier classification. Lexicons that are operator-internal \
                 infrastructure may set `audience_exempt: true` in their manifest entry with an \
                 `exemption_reason` documenting the substrate-class capability or oracle that \
                 gates access."
            ),
            BuildError::MissingExemptionReason(nsid) => write!(
                f,
                "lexicon `{nsid}` is marked `audience_exempt: true` but does not document an \
                 `exemption_reason`. §5.4 requires every exempt entry to identify the \
                 substrate-class capability or oracle that gates access."
            ),
            BuildError::AudienceMechanismMustBeExempt { nsid } => write!(
                f,
                "lexicon `{nsid}` is the audience mechanism itself; the manifest entry must \
                 set `audience_exempt: true` with an `exemption_reason` per §5.4."
            ),
            BuildError::UnknownTier(t) => {
                write!(f, "unrecognized tier value `{t}` (expected `public` or `private`)")
            }
        }
    }
}

// Suppress dead-code on `Read` import in some build paths.
#[allow(dead_code)]
fn _suppress_read_unused() {
    let _: Option<&dyn Read> = None;
}

// Suppress dead-code on `Write` import for future use.
#[allow(dead_code)]
fn _suppress_write_unused() {
    let _: Option<&dyn Write> = None;
}
