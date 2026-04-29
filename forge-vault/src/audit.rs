//! Append-only audit log for vault operations.
//!
//! Writes structured JSON-lines events to `.forge-index/audit.jsonl`.
//! Each line is a self-contained JSON object with a UTC timestamp, event
//! kind, and optional metadata.
//!
//! The log is designed to be tamper-evident (append-only, no edits) and
//! lightweight (one `OpenOptions::append` call per event). It is NOT
//! encrypted — local-only threat model applies.
//!
//! ## Rotation
//!
//! When the log exceeds [`MAX_LOG_SIZE_BYTES`], the current file is
//! renamed to `audit-<timestamp>.jsonl` and a fresh file is started.

use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Filename of the active audit log inside `.forge-index/`.
const AUDIT_FILE: &str = "audit.jsonl";
/// Maximum log size before rotation (1 MB).
const MAX_LOG_SIZE_BYTES: u64 = 1_048_576;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// Discriminant for the kind of security-relevant event logged to the audit trail.
///
/// Each variant represents a distinct category of operation (open, read, write, delete, search,
/// integrity). Use [`AuditLog::log`] to record an event along with an optional detail string
/// (e.g. the note path, the query text).
///
/// Events are serialized to JSON-lines in the audit log, sorted by timestamp, for tamper-evident
/// tracking and forensic analysis.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEvent {
    /// Vault opened (full or incremental index).
    ///
    /// Logged when [`VaultStore::open`] initializes the index. Distinguishes between
    /// cold-start (full re-index) and warm-start (incremental) for performance analysis.
    VaultOpened,
    /// A note was read from disk.
    ///
    /// Logged on note open for viewing or editing. Useful for access logs and data exfiltration
    /// detection.
    NoteRead,
    /// A note was saved or modified.
    ///
    /// Logged after successful write to disk. Captures authorship and timing for
    /// version history and accountability.
    NoteSaved,
    /// A note was deleted.
    ///
    /// Logged when a note is removed from disk. Critical for compliance and forensics;
    /// helps detect unauthorized deletion.
    NoteDeleted,
    /// A full-text search was executed.
    ///
    /// Logged for each search query. Used to detect information disclosure patterns
    /// and analyze vault usage.
    SearchQuery,
    /// The manifest was saved.
    ///
    /// Logged when the incremental index is synced to disk. Useful for tracking
    /// index freshness and consistency checks.
    ManifestSaved,
    /// HMAC verification failed on manifest load.
    ///
    /// Logged when manifest integrity check fails, indicating possible tampering
    /// or corruption. Triggers automatic full re-index.
    HmacFailed,
}

/// A single audit log entry.
#[derive(Debug, Serialize)]
struct AuditEntry<'a> {
    /// UTC ISO-8601 timestamp.
    ts: String,
    /// Event kind.
    event: AuditEvent,
    /// Optional human-readable detail (path, query, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// AuditLog
// ---------------------------------------------------------------------------

/// Append-only audit logger for a single vault.
///
/// Tracks every security-sensitive vault operation (open, read, write, delete, search) to a
/// persistent JSONL file at `.forge-index/audit.jsonl`. Each line is a self-contained JSON
/// object with UTC timestamp, event kind, and optional metadata (note path, query text, etc.).
///
/// The audit trail is append-only — no edits or deletions of existing entries — making it
/// tamper-evident for forensic analysis. On write failure, errors are logged but never
/// propagated, ensuring audit delays do not block vault operations.
///
/// Cheap to clone (just holds a `PathBuf`). Safe to call [`log`](Self::log) from any context.
#[derive(Debug, Clone)]
pub struct AuditLog {
    /// Path to `.forge-index/audit.jsonl`.
    path: PathBuf,
}

impl AuditLog {
    /// Create a new audit logger for the given vault root.
    ///
    /// The logger is constructed but does NOT create the audit file itself — that happens
    /// lazily on the first write. This design ensures minimal overhead if audit is never used.
    ///
    /// # Arguments
    /// - `vault_root` — absolute path to the vault directory. The audit log is stored at
    ///   `<vault_root>/.forge-index/audit.jsonl`.
    pub fn new(vault_root: &Path) -> Self {
        Self {
            path: vault_root.join(".forge-index").join(AUDIT_FILE),
        }
    }

    /// Append an event to the audit log with optional metadata.
    ///
    /// Errors are logged but never propagated — audit failures must not block normal vault
    /// operations. This fire-and-forget design prioritizes vault responsiveness over audit
    /// completeness (acceptable for local-only threat model).
    ///
    /// # Arguments
    /// - `event` — the kind of operation being logged.
    /// - `detail` — optional human-readable metadata (note path, search query, error message).
    ///
    /// # Thread Safety
    /// Safe to call from multiple threads. The underlying file write is serialized via the
    /// OS file handle.
    pub fn log(&self, event: AuditEvent, detail: Option<&str>) {
        if let Err(e) = self.try_log(event, detail) {
            tracing::warn!(error = %e, "failed to write audit log entry");
        }
    }

    /// Internal: attempt to write a log entry.
    fn try_log(&self, event: AuditEvent, detail: Option<&str>) -> std::io::Result<()> {
        // Rotate if needed.
        self.rotate_if_needed()?;

        // Ensure parent dir exists.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let entry = AuditEntry {
            ts: Utc::now().to_rfc3339(),
            event,
            detail,
        };

        let mut line = serde_json::to_string(&entry).map_err(std::io::Error::other)?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;

        Ok(())
    }

    /// Rotate the log file if it exceeds the size threshold.
    fn rotate_if_needed(&self) -> std::io::Result<()> {
        let meta = match fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return Ok(()), // File does not exist yet.
        };

        if meta.len() >= MAX_LOG_SIZE_BYTES {
            let stamp = Utc::now().format("%Y%m%dT%H%M%S");
            let rotated = self.path.with_file_name(format!("audit-{stamp}.jsonl"));
            fs::rename(&self.path, &rotated)?;
            tracing::info!(rotated_to = %rotated.display(), "audit log rotated");
        }

        Ok(())
    }
}
