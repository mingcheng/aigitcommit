/*!
 * Copyright (c) 2026 mingcheng <mingcheng@apache.org>
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * Lightweight on-disk cache for OpenAI responses.
 *
 * The cache key is derived from the inputs that influence the API request
 * (model name, system prompt, staged diff and recent commit logs). When the
 * staged diff and the surrounding context have not changed, the previously
 * generated commit message can be reused without contacting the API.
 *
 * File: cache.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2026-05-07 11:24:15
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-05-07 11:44:53
 */

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

/// Subdirectory name (under `<repo>/.git/`) used to store cached responses.
const CACHE_DIR_NAME: &str = "aigitcommit-cache";

/// File-based cache for raw OpenAI chat responses.
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    /// Create a new cache rooted under the given git directory.
    ///
    /// The cache directory is created lazily on first write, so this constructor
    /// never fails even if the directory does not yet exist.
    pub fn new(git_dir: &Path) -> Self {
        Self {
            dir: git_dir.join(CACHE_DIR_NAME),
        }
    }

    /// Build a stable cache key from the request-relevant inputs.
    ///
    /// Uses 64-bit FNV-1a so the same inputs always produce the same key
    /// regardless of the standard library's hashing implementation.
    pub fn build_key(
        model: &str,
        system_prompt: &str,
        diffs: &[String],
        logs: &[String],
    ) -> String {
        let mut hasher = Fnv1a64::new();
        hasher.write(model.as_bytes());
        hasher.write(b"\0");
        hasher.write(system_prompt.as_bytes());
        hasher.write(b"\0");
        for d in diffs {
            hasher.write(d.as_bytes());
            hasher.write(b"\n");
        }
        hasher.write(b"\0");
        for l in logs {
            hasher.write(l.as_bytes());
            hasher.write(b"\n");
        }
        format!("{:016x}", hasher.finish())
    }

    /// Read a cached entry by key. Returns `None` if the entry does not exist
    /// or cannot be read.
    pub fn get(&self, key: &str) -> Option<String> {
        let path = self.entry_path(key);
        match fs::read_to_string(&path) {
            Ok(content) => {
                debug!("cache hit: {}", path.display());
                Some(content)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                trace!("cache miss for key {}", key);
                None
            }
            Err(e) => {
                warn!("failed to read cache entry {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Persist a cache entry. Errors are logged but not propagated, since a
    /// failed cache write should never break commit message generation.
    pub fn put(&self, key: &str, value: &str) {
        if let Err(e) = fs::create_dir_all(&self.dir) {
            warn!(
                "failed to create cache directory {}: {}",
                self.dir.display(),
                e
            );
            return;
        }

        let path = self.entry_path(key);
        match fs::write(&path, value) {
            Ok(()) => trace!("wrote cache entry {}", path.display()),
            Err(e) => warn!("failed to write cache entry {}: {}", path.display(), e),
        }
    }

    /// Remove the entire cache directory. Returns the number of files removed.
    pub fn clear(&self) -> Result<usize, Box<dyn Error>> {
        if !self.dir.exists() {
            return Ok(0);
        }

        let mut count = 0usize;
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::remove_file(entry.path())?;
                count += 1;
            }
        }

        // Best-effort removal of the now-empty directory.
        let _ = fs::remove_dir(&self.dir);
        Ok(count)
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        self.dir.join(key)
    }
}

/// Minimal FNV-1a 64-bit hasher. Stable across platforms and Rust versions.
struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET,
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut state = self.state;
        for b in bytes {
            state ^= *b as u64;
            state = state.wrapping_mul(Self::PRIME);
        }
        self.state = state;
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_key_is_stable_and_sensitive() {
        let diffs = vec!["diff --git a/x b/x".to_string(), "+hello".to_string()];
        let logs = vec!["initial commit".to_string()];
        let k1 = Cache::build_key("gpt-5", "sys", &diffs, &logs);
        let k2 = Cache::build_key("gpt-5", "sys", &diffs, &logs);
        assert_eq!(k1, k2);

        let k3 = Cache::build_key("gpt-5", "sys", &diffs, &["other".to_string()]);
        assert_ne!(k1, k3);

        let k4 = Cache::build_key("gpt-4", "sys", &diffs, &logs);
        assert_ne!(k1, k4);
    }

    #[test]
    fn get_returns_none_for_missing_entry() {
        let tmp =
            std::env::temp_dir().join(format!("aigitcommit-cache-test-{}", std::process::id()));
        let cache = Cache::new(&tmp);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn clear_removes_entries_and_returns_count() {
        let tmp = std::env::temp_dir()
            .join(format!("aigitcommit-cache-clear-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let cache = Cache::new(&tmp);
        cache.put("a", "1");
        cache.put("b", "2");
        cache.put("c", "3");
        let removed = cache.clear().unwrap();
        assert_eq!(removed, 3);
        // Subsequent clear on a non-existent dir is a no-op.
        assert_eq!(cache.clear().unwrap(), 0);
    }

    #[test]
    fn build_key_format_is_16_hex_chars() {
        let k = Cache::build_key("m", "s", &[], &[]);
        assert_eq!(k.len(), 16);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
