//! Vault manifest — persistent metadata for incremental indexing.
//!
//! Stores `path -> (NoteId, mtime)` so that [`VaultStore::open`] can skip
//! unchanged files and only re-index what actually changed on disk.
//!
//! The manifest is saved as `.forge-index/manifest.json` alongside the Tantivy
//! index.  It is always considered a cache: if missing or corrupt the vault
//! falls back to a full re-index (safe but slower).
//!
//! ## Integrity
//!
//! The manifest is signed with HMAC-SHA256 on every save. The signature is
//! stored in `.forge-index/manifest.sig` (hex-encoded). A per-vault secret
//! key is generated on first open and stored in `.forge-index/.hmac-key`.
//! On load, the signature is verified — a mismatch triggers a full re-index
//! (same fallback as a corrupt or missing manifest).

use forge_core::NoteId;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Persisted per-note metadata stored in the vault manifest.
///
/// Captures the essential state needed for incremental indexing: the note's stable ID,
/// its last-known on-disk modification time, and cached wikilink targets. Allows
/// [`VaultStore::open`] to skip unchanged notes by comparing on-disk mtime against the
/// manifest, dramatically speeding up warm-start opens.
///
/// Wikilinks are cached to avoid re-reading and re-parsing note bodies to rebuild the
/// knowledge graph, which is useful for graph queries that don't require fresh content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    /// Stable identifier that survives restarts.
    ///
    /// Assigned on first index, unique within the vault. Used as the primary key in
    /// the Tantivy full-text index and in graph relationships.
    pub id: NoteId,
    /// Last-modified timestamp (seconds since UNIX epoch).
    ///
    /// Set when the note is indexed. Compared against on-disk `mtime` on subsequent opens
    /// to detect changes without re-reading the file. Split into two fields for portability
    /// (avoids `SystemTime` serialization issues across platforms).
    pub mtime_secs: u64,
    /// Last-modified timestamp (nanoseconds component, 0–999,999,999).
    pub mtime_nanos: u32,
    /// Cached wikilink targets (lowercase stems) extracted from the note body.
    ///
    /// Examples: from `[[Database Design]]` or `[[Database Design|custom label]]`,
    /// extract `"database-design"` as the wikilink stem. Stored so the knowledge graph
    /// can be rebuilt without re-reading any files (useful for cold-start graph queries).
    /// Default empty vec on old manifests (missing field = no wikilinks cached).
    #[serde(default)]
    pub wikilinks: Vec<String>,
}

impl NoteEntry {
    /// Construct a new `NoteEntry` from a note ID and on-disk modification time.
    ///
    /// Converts the `SystemTime` to seconds + nanoseconds since UNIX epoch for compact,
    /// portable serialization. Wikilinks list is initialized empty; use [`with_wikilinks`]
    /// if pre-extracted links are available.
    ///
    /// # Arguments
    /// - `id` — the note's stable identifier (e.g., assigned by the indexer).
    /// - `mtime` — typically obtained from `fs::metadata(path).modified()`. If the system
    ///   time is before UNIX epoch (rare), defaults to epoch itself.
    ///
    /// [`with_wikilinks`]: Self::with_wikilinks
    pub fn new(id: NoteId, mtime: SystemTime) -> Self {
        let dur = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            id,
            mtime_secs: dur.as_secs(),
            mtime_nanos: dur.subsec_nanos(),
            wikilinks: Vec::new(),
        }
    }

    /// Construct a new `NoteEntry` with pre-extracted wikilinks.
    ///
    /// Preferred constructor when wikilinks have already been parsed from the note body.
    /// Avoids re-parsing during future graph queries. If the list is empty, behavior is
    /// identical to [`new`].
    ///
    /// # Arguments
    /// - `id` — the note's stable identifier.
    /// - `mtime` — the note's on-disk modification time.
    /// - `wikilinks` — lowercase stems of wikilink targets (e.g., `["database-design", "index"]`).
    ///
    /// [`new`]: Self::new
    pub fn with_wikilinks(id: NoteId, mtime: SystemTime, wikilinks: Vec<String>) -> Self {
        let dur = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            id,
            mtime_secs: dur.as_secs(),
            mtime_nanos: dur.subsec_nanos(),
            wikilinks,
        }
    }

    /// Reconstruct the [`SystemTime`] from stored seconds and nanoseconds.
    ///
    /// Inverse of the conversion performed in [`new`]. Used to compare the manifest's
    /// recorded mtime against the on-disk mtime to detect if the note has changed.
    ///
    /// # Example
    /// ```ignore
    /// let disk_mtime = fs::metadata(path).modified()?;
    /// if disk_mtime != entry.mtime() {
    ///     // Note has changed; re-index it.
    /// }
    /// ```
    ///
    /// [`new`]: Self::new
    pub fn mtime(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::new(self.mtime_secs, self.mtime_nanos)
    }
}

/// The vault manifest — persistent index cache mapping file paths to note metadata.
///
/// Enables incremental indexing: instead of re-reading and re-parsing every note on every open,
/// the manifest stores `path -> (id, mtime, wikilinks)` so that [`VaultStore::open`] can
/// skip unchanged files (by comparing on-disk mtime) and only index what changed. Acts as a cache:
/// if the manifest is missing, corrupt, or its HMAC fails, the vault safely falls back to a full
/// re-index (slower but correct).
///
/// The manifest is persisted as JSON at `.forge-index/manifest.json` and signed with
/// HMAC-SHA256 at `.forge-index/manifest.sig` to detect tampering or corruption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version for backward compatibility.
    ///
    /// Bumped when the `NoteEntry` structure changes in a way that breaks deserialization.
    /// On version mismatch, the manifest is discarded and a full re-index is triggered.
    /// Currently at `MANIFEST_VERSION = 1`.
    pub version: u32,
    /// Known notes, keyed by their absolute file path as a `String`.
    ///
    /// Each entry contains the note's stable ID, last-indexed mtime, and cached wikilinks.
    /// Used to detect file additions, modifications, and deletions via [`diff`].
    pub notes: HashMap<String, NoteEntry>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            notes: HashMap::new(),
        }
    }
}

/// Current manifest schema version.
const MANIFEST_VERSION: u32 = 1;

/// Filename of the manifest inside `.forge-index/`.
const MANIFEST_FILE: &str = "manifest.json";
/// Filename of the HMAC signature alongside the manifest.
const SIGNATURE_FILE: &str = "manifest.sig";
/// Filename of the per-vault HMAC secret key.
const HMAC_KEY_FILE: &str = ".hmac-key";
/// Length of the HMAC key in bytes (256-bit).
const HMAC_KEY_LEN: usize = 32;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// HMAC helpers
// ---------------------------------------------------------------------------

/// Load or create the per-vault HMAC key from `.forge-index/.hmac-key`.
///
/// The key is a 32-byte random secret generated once at vault creation time.
/// If the key file is missing or unreadable, a new key is generated and saved.
fn load_or_create_hmac_key(vault_root: &Path) -> std::io::Result<Vec<u8>> {
    let dir = vault_root.join(".forge-index");
    std::fs::create_dir_all(&dir)?;
    let key_path = dir.join(HMAC_KEY_FILE);

    // Try loading existing key.
    if let Ok(hex_str) = std::fs::read_to_string(&key_path) {
        let hex_str = hex_str.trim();
        if let Ok(key) = hex::decode(hex_str) {
            if key.len() == HMAC_KEY_LEN {
                return Ok(key);
            }
        }
        tracing::warn!("corrupt HMAC key file — regenerating");
    }

    // Generate a new key.
    use rand::RngCore;
    let mut key = vec![0u8; HMAC_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);

    let hex_str = hex::encode(&key);
    std::fs::write(&key_path, hex_str)?;

    // Restrict permissions on the key file (Unix only).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
    }

    tracing::info!("generated new HMAC key for vault integrity");
    Ok(key)
}

/// Compute HMAC-SHA256 over the manifest JSON data.
fn compute_hmac(key: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    hex::encode(mac.finalize().into_bytes())
}

/// Verify an HMAC-SHA256 signature against the manifest data.
fn verify_hmac(key: &[u8], data: &[u8], expected_hex: &str) -> bool {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    let expected = match hex::decode(expected_hex.trim()) {
        Ok(v) => v,
        Err(_) => return false,
    };
    mac.verify_slice(&expected).is_ok()
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

impl Manifest {
    /// Load the manifest from disk, verifying schema and integrity.
    ///
    /// Reads from `<vault_root>/.forge-index/manifest.json` and validates:
    /// 1. File exists and is readable.
    /// 2. JSON deserializes to a valid [`Manifest`].
    /// 3. Schema version matches (currently `MANIFEST_VERSION = 1`).
    /// 4. HMAC-SHA256 signature matches (if a signature file exists).
    ///
    /// Returns `None` if any check fails. The caller should fall back to a full re-index.
    /// This is safe by design: the manifest is a cache, not ground truth.
    ///
    /// # Arguments
    /// - `vault_root` — absolute path to the vault directory.
    ///
    /// # Signature Verification
    /// On first open, no signature file exists and the manifest is trusted (logged as info).
    /// On subsequent opens, the signature is verified to detect tampering or corruption.
    pub fn load(vault_root: &Path) -> Option<Self> {
        let dir = vault_root.join(".forge-index");
        let path = dir.join(MANIFEST_FILE);
        let data = std::fs::read_to_string(&path).ok()?;
        let manifest: Self = serde_json::from_str(&data).ok()?;

        if manifest.version != MANIFEST_VERSION {
            tracing::warn!(
                found = manifest.version,
                expected = MANIFEST_VERSION,
                "manifest version mismatch — full re-index required",
            );
            return None;
        }

        // Verify HMAC integrity if a signature file exists.
        let sig_path = dir.join(SIGNATURE_FILE);
        if sig_path.exists() {
            let key = load_or_create_hmac_key(vault_root).ok()?;
            let sig_hex = std::fs::read_to_string(&sig_path).ok()?;
            if !verify_hmac(&key, data.as_bytes(), &sig_hex) {
                tracing::warn!("manifest HMAC verification failed — full re-index required");
                return None;
            }
            tracing::debug!("manifest HMAC verified");
        } else {
            tracing::info!("no manifest signature found — will sign on next save");
        }

        Some(manifest)
    }

    /// Persist the manifest to disk and sign it with HMAC-SHA256.
    ///
    /// Writes to `<vault_root>/.forge-index/manifest.json` (pretty-printed JSON) and
    /// signs with a per-vault HMAC-SHA256 key stored at `.forge-index/.hmac-key`.
    /// The signature hex-string is written to `.forge-index/manifest.sig`.
    ///
    /// Creates `.forge-index/` directory if it does not exist. On Unix systems, restricts
    /// the HMAC key file to mode `0o600` (owner read-write only) for basic secrets protection.
    ///
    /// # Arguments
    /// - `vault_root` — absolute path to the vault directory.
    ///
    /// # Errors
    /// Returns I/O errors (file write, permission issues, HMAC key generation failures).
    pub fn save(&self, vault_root: &Path) -> std::io::Result<()> {
        let dir = vault_root.join(".forge-index");
        std::fs::create_dir_all(&dir)?;

        let data = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;

        // Write manifest.
        let path = dir.join(MANIFEST_FILE);
        std::fs::write(&path, &data)?;

        // Sign and write signature.
        let key = load_or_create_hmac_key(vault_root)?;
        let sig = compute_hmac(&key, data.as_bytes());
        let sig_path = dir.join(SIGNATURE_FILE);
        std::fs::write(&sig_path, &sig)?;

        tracing::debug!("manifest saved and signed");
        Ok(())
    }

    /// Lookup a note entry by absolute path.
    ///
    /// # Arguments
    /// - `path` — absolute file path of the note.
    ///
    /// # Returns
    /// A reference to the `NoteEntry` if found, `None` otherwise.
    ///
    /// # Example
    /// ```ignore
    /// let entry = manifest.get(Path::new("/home/user/vault/notes/index.md"))?;
    /// println!("Note ID: {}, cached wikilinks: {:?}", entry.id, entry.wikilinks);
    /// ```
    pub fn get(&self, path: &Path) -> Option<&NoteEntry> {
        self.notes.get(&path.display().to_string())
    }

    /// Insert or update a note entry.
    ///
    /// If the path already exists in the manifest, its entry is replaced.
    /// Call [`save`] to persist changes to disk.
    ///
    /// # Arguments
    /// - `path` — absolute file path of the note.
    /// - `entry` — the `NoteEntry` with the note's ID, mtime, and cached wikilinks.
    ///
    /// [`save`]: Self::save
    pub fn upsert(&mut self, path: &Path, entry: NoteEntry) {
        self.notes.insert(path.display().to_string(), entry);
    }

    /// Remove a note entry by path.
    ///
    /// Has no effect if the path is not in the manifest.
    /// Call [`save`] to persist changes to disk.
    ///
    /// # Arguments
    /// - `path` — absolute file path of the note.
    ///
    /// [`save`]: Self::save
    pub fn remove(&mut self, path: &Path) {
        self.notes.remove(&path.display().to_string());
    }
}

// ---------------------------------------------------------------------------
// Diff engine
// ---------------------------------------------------------------------------

/// Result of comparing the on-disk file tree against the vault manifest.
///
/// Produced by [`diff`], used by [`VaultStore::open`] to decide which files need indexing.
/// Files are categorized into four disjoint sets: added (never seen), modified (mtime changed),
/// deleted (in manifest but not on disk), and unchanged (mtime matches).
///
/// A "clean" diff (no added/modified/deleted) means the vault is already fully indexed and
/// the existing Tantivy index can be reused as-is.
#[derive(Debug, Default)]
pub struct VaultDiff {
    /// Files not in the manifest (need fresh indexing).
    ///
    /// These are new files added to the vault since the last index. Typical case: user
    /// creates a new note file while the vault is open.
    pub added: Vec<PathBuf>,
    /// Files whose on-disk mtime differs from the manifest (need re-indexing).
    ///
    /// The mtime changed means the file was edited. For large vaults, only re-indexing
    /// these modified files (instead of all files) provides the speed benefit of incremental
    /// indexing.
    pub modified: Vec<PathBuf>,
    /// Files in the manifest but no longer on disk (need removal).
    ///
    /// The note was deleted externally (e.g., via the file manager). Must be removed from
    /// the Tantivy index and the manifest to keep them consistent.
    pub deleted: Vec<PathBuf>,
    /// Files whose mtime matches the manifest (skip re-indexing).
    ///
    /// These notes have not changed since the last index. Their Tantivy index entries and
    /// metadata are assumed still valid and can be reused.
    pub unchanged: Vec<PathBuf>,
}

impl VaultDiff {
    /// True if the vault is clean: no added, modified, or deleted files.
    ///
    /// A clean diff means the on-disk file tree exactly matches the manifest's view,
    /// so the existing Tantivy index is fully up-to-date. Useful for short-circuit logic:
    /// if `is_clean()`, [`VaultStore::open`] can skip the entire indexing phase and reuse
    /// the previous index.
    ///
    /// # Returns
    /// `true` iff `added.is_empty() && modified.is_empty() && deleted.is_empty()`.
    pub fn is_clean(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Count of files requiring action: added + modified + deleted.
    ///
    /// Useful for progress reporting and performance metrics. The `unchanged` count
    /// is implicit: `total_files = dirty_count() + unchanged.len()`.
    ///
    /// # Example
    /// ```ignore
    /// let diff = diff(&on_disk_paths, &manifest);
    /// println!("Dirty: {}, Unchanged: {}", diff.dirty_count(), diff.unchanged.len());
    /// ```
    pub fn dirty_count(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

/// Compute a [`VaultDiff`] by comparing on-disk markdown files against the manifest.
///
/// Categorizes each on-disk file as: added (not in manifest), modified (mtime differs),
/// or unchanged (mtime matches). Also detects deletions (files in manifest but not on disk).
///
/// This is the core of incremental indexing: only files in the added or modified sets
/// need to be re-indexed; unchanged files can reuse their cached index entries.
///
/// # Arguments
/// - `on_disk` — list of absolute file paths found on disk (typically `*.md` notes).
/// - `manifest` — the current vault manifest (from a previous open or `Manifest::default()`).
///
/// # Returns
/// A [`VaultDiff`] with four disjoint sets: added, modified, deleted, unchanged.
/// Summing their lengths equals the vault's total file count.
pub fn diff(on_disk: &[PathBuf], manifest: &Manifest) -> VaultDiff {
    let mut result = VaultDiff::default();

    // Track which manifest entries we've seen (to detect deletions).
    let mut seen: HashMap<&str, bool> =
        manifest.notes.keys().map(|k| (k.as_str(), false)).collect();

    for path in on_disk {
        let key = path.display().to_string();

        if let Some(entry) = manifest.notes.get(&key) {
            // Mark as seen.
            if let Some(v) = seen.get_mut(key.as_str()) {
                *v = true;
            }

            // Compare mtime.
            let disk_mtime = std::fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if disk_mtime == entry.mtime() {
                result.unchanged.push(path.clone());
            } else {
                result.modified.push(path.clone());
            }
        } else {
            result.added.push(path.clone());
        }
    }

    // Files in manifest but not on disk → deleted.
    for (path_str, was_seen) in &seen {
        if !was_seen {
            result.deleted.push(PathBuf::from(path_str));
        }
    }

    result
}
