//! Virtual filesystem abstraction for vault I/O.
//!
//! The [`StorageBackend`] trait decouples `VaultStore` from the real filesystem,
//! enabling:
//! - **Unit tests** using [`MemoryFs`] without touching disk
//! - **Future backends**: S3, SQLite, encrypted volumes, etc.
//!
//! ## Migration path
//! `VaultStore` currently uses `std::fs` directly.  To adopt this trait:
//! 1. Add a generic parameter `S: StorageBackend` to `VaultStore`
//! 2. Replace `std::fs::read_to_string(&path)` with `self.storage.read_to_string(&path)`
//! 3. Replace `std::fs::read_dir(...)` with `self.storage.read_dir(...)`
//! 4. Wire `RealFs` in production, `MemoryFs` in tests

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Minimal metadata returned by the storage backend.
#[derive(Debug, Clone)]
pub struct FileMeta {
    /// Last modification time.
    pub modified: SystemTime,
    /// File size in bytes.
    pub len: u64,
    /// True if entry is a directory.
    pub is_dir: bool,
}

/// A single directory entry.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Absolute path of the entry.
    pub path: PathBuf,
    /// Entry file name (last component).
    pub name: String,
    /// True if the entry is a directory.
    pub is_dir: bool,
}

/// Abstraction over filesystem operations used by `VaultStore`.
///
/// All paths are **absolute**.  Implementations must handle path resolution
/// internally.
///
/// This trait is intentionally synchronous to match `VaultStore::open`
/// (which runs inside `spawn_blocking`).  An async variant can be added
/// later via a separate trait if needed.
pub trait StorageBackend: Send + Sync {
    /// Read the entire contents of a file as a UTF-8 string.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Write `content` to a file, creating it if it does not exist,
    /// truncating it if it does.
    fn write(&self, path: &Path, content: &str) -> io::Result<()>;

    /// Return metadata for a file or directory.
    fn metadata(&self, path: &Path) -> io::Result<FileMeta>;

    /// List the immediate children of a directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;

    /// Check whether a path exists.
    fn exists(&self, path: &Path) -> bool;

    /// Create a directory and all its parents.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Rename (move) a file or directory.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Remove a directory and all its contents recursively.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
}

// ---------------------------------------------------------------------------
// Real filesystem implementation
// ---------------------------------------------------------------------------

/// Production implementation backed by `std::fs`.
#[derive(Debug, Clone, Default)]
pub struct RealFs;

impl StorageBackend for RealFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, content: &str) -> io::Result<()> {
        std::fs::write(path, content)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMeta> {
        let m = std::fs::metadata(path)?;
        Ok(FileMeta {
            modified: m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            len: m.len(),
            is_dir: m.is_dir(),
        })
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            entries.push(DirEntry { path, name, is_dir });
        }
        Ok(entries)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation (for tests)
// ---------------------------------------------------------------------------

/// In-memory filesystem for unit testing.
///
/// Thread-safe via `Arc<Mutex<...>>`.  Directories are tracked implicitly:
/// any path that is a prefix of a file path is considered a directory.
///
/// # Example
/// ```
/// use forge_vault::storage::{MemoryFs, StorageBackend};
/// use std::path::Path;
///
/// let fs = MemoryFs::new();
/// fs.write(Path::new("/vault/note.md"), "# Hello").unwrap();
/// assert!(fs.exists(Path::new("/vault/note.md")));
/// assert_eq!(fs.read_to_string(Path::new("/vault/note.md")).unwrap(), "# Hello");
/// ```
#[derive(Debug, Clone)]
pub struct MemoryFs {
    /// file path -> (content, modified_at)
    files: Arc<Mutex<HashMap<PathBuf, (String, SystemTime)>>>,
    /// Explicitly created directories.
    dirs: Arc<Mutex<std::collections::HashSet<PathBuf>>>,
}

impl MemoryFs {
    /// Create a new empty in-memory filesystem.
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            dirs: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

impl Default for MemoryFs {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for MemoryFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        files
            .get(path)
            .map(|(content, _)| content.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{}", path.display())))
    }

    fn write(&self, path: &Path, content: &str) -> io::Result<()> {
        // Auto-create parent directories.
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent)?;
        }
        let mut files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        files.insert(path.to_path_buf(), (content.to_string(), SystemTime::now()));
        Ok(())
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMeta> {
        let files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        if let Some((content, modified)) = files.get(path) {
            return Ok(FileMeta {
                modified: *modified,
                len: content.len() as u64,
                is_dir: false,
            });
        }
        drop(files);

        let dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");
        if dirs.contains(path) {
            return Ok(FileMeta {
                modified: SystemTime::now(),
                len: 0,
                is_dir: true,
            });
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}", path.display()),
        ))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        let dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");

        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect immediate child files.
        for file_path in files.keys() {
            if let Some(parent) = file_path.parent() {
                if parent == path {
                    let name = file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if seen.insert(file_path.clone()) {
                        entries.push(DirEntry {
                            path: file_path.clone(),
                            name,
                            is_dir: false,
                        });
                    }
                }
            }
        }

        // Collect immediate child directories.
        for dir_path in dirs.iter() {
            if let Some(parent) = dir_path.parent() {
                if parent == path && !seen.contains(dir_path) {
                    let name = dir_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    seen.insert(dir_path.clone());
                    entries.push(DirEntry {
                        path: dir_path.clone(),
                        name,
                        is_dir: true,
                    });
                }
            }
        }

        Ok(entries)
    }

    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        if files.contains_key(path) {
            return true;
        }
        drop(files);
        let dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");
        dirs.contains(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");
        // Insert this dir and all ancestors.
        let mut current = path.to_path_buf();
        loop {
            dirs.insert(current.clone());
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => break,
            }
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        if let Some(entry) = files.remove(from) {
            files.insert(to.to_path_buf(), entry);
            return Ok(());
        }
        drop(files);

        // Rename directory: move all files under `from` to `to`.
        let mut files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        let keys_to_move: Vec<PathBuf> = files
            .keys()
            .filter(|p| p.starts_with(from))
            .cloned()
            .collect();

        for key in keys_to_move {
            if let Some(entry) = files.remove(&key) {
                let suffix = key
                    .strip_prefix(from)
                    .expect("key must start with `from` (filtered by starts_with)");
                let new_key = to.join(suffix);
                files.insert(new_key, entry);
            }
        }

        let mut dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");
        let dirs_to_move: Vec<PathBuf> = dirs
            .iter()
            .filter(|p| p.starts_with(from))
            .cloned()
            .collect();
        for d in dirs_to_move {
            dirs.remove(&d);
            let suffix = d
                .strip_prefix(from)
                .expect("dir must start with `from` (filtered by starts_with)");
            dirs.insert(to.join(suffix));
        }

        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut files = self.files.lock().expect("MemoryFs::files mutex poisoned");
        files.retain(|p, _| !p.starts_with(path));
        drop(files);

        let mut dirs = self.dirs.lock().expect("MemoryFs::dirs mutex poisoned");
        dirs.retain(|p| !p.starts_with(path));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn memory_fs_write_and_read() {
        let fs = MemoryFs::new();
        let path = Path::new("/vault/notes/hello.md");
        fs.write(path, "# Hello World").unwrap();

        assert!(fs.exists(path));
        assert_eq!(fs.read_to_string(path).unwrap(), "# Hello World");
    }

    #[test]
    fn memory_fs_read_dir() {
        let fs = MemoryFs::new();
        fs.write(Path::new("/vault/a.md"), "a").unwrap();
        fs.write(Path::new("/vault/b.md"), "b").unwrap();
        fs.write(Path::new("/vault/sub/c.md"), "c").unwrap();

        let entries = fs.read_dir(Path::new("/vault")).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.md"));
        assert!(names.contains(&"b.md"));
        assert!(names.contains(&"sub"));
    }

    #[test]
    fn memory_fs_rename_file() {
        let fs = MemoryFs::new();
        let from = Path::new("/vault/old.md");
        let to = Path::new("/vault/new.md");
        fs.write(from, "content").unwrap();
        fs.rename(from, to).unwrap();

        assert!(!fs.exists(from));
        assert_eq!(fs.read_to_string(to).unwrap(), "content");
    }

    #[test]
    fn memory_fs_remove_dir_all() {
        let fs = MemoryFs::new();
        fs.write(Path::new("/vault/folder/a.md"), "a").unwrap();
        fs.write(Path::new("/vault/folder/b.md"), "b").unwrap();
        fs.write(Path::new("/vault/keep.md"), "keep").unwrap();

        fs.remove_dir_all(Path::new("/vault/folder")).unwrap();

        assert!(!fs.exists(Path::new("/vault/folder/a.md")));
        assert!(!fs.exists(Path::new("/vault/folder/b.md")));
        assert!(fs.exists(Path::new("/vault/keep.md")));
    }

    #[test]
    fn real_fs_basic_operations() {
        let fs = RealFs;
        // Just verify the trait is object-safe and callable.
        let _: &dyn StorageBackend = &fs;
    }
}
