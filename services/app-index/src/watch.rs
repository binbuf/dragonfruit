// SPDX-License-Identifier: MIT
//! `.desktop` directory watching (T-14.1b).
//!
//! The index only stays live if an install/uninstall/update reaches it. The
//! design forbids polling at idle: the service watches the applications
//! directories with inotify and re-scans on a monitor event. This mirrors
//! `files-core`'s sanctioned inotify fallback for the same reason (no
//! GIO/GVfs `GFileMonitor` in the pinned toolchain), but watches a whole tree
//! recursively because desktop file ids are built from nested directories.
//!
//! A monitor event only says "something under a watched directory changed", so
//! the callback asks the index to re-scan and diff; the diff is what yields
//! the install/uninstall/update events. Events are debounced by a short quiet
//! period, because one install is a burst of create/write/close events and the
//! expensive step is the diff, not the notification.

use std::path::PathBuf;
use std::time::Duration;

/// The quiet period after the last inotify event before re-scanning.
pub const DEBOUNCE_MS: u64 = 150;

/// The watch mask: entry and directory creation/removal, renames, writes, and
/// attribute changes all mean "the corpus may have changed".
#[cfg(target_os = "linux")]
const WATCH_MASK: u32 = libc::IN_CREATE
    | libc::IN_DELETE
    | libc::IN_MOVED_FROM
    | libc::IN_MOVED_TO
    | libc::IN_MODIFY
    | libc::IN_CLOSE_WRITE
    | libc::IN_ATTRIB
    | libc::IN_DELETE_SELF
    | libc::IN_MOVE_SELF;

/// A recursive inotify watch over a set of directores. [`DirectoryWatcher::new`]
/// establishes the watches synchronously, so an event that races the caller's
/// first write is not missed; [`DirectoryWatcher::run`] then blocks.
#[cfg(target_os = "linux")]
pub struct DirectoryWatcher {
    fd: libc::c_int,
    dirs: Vec<PathBuf>,
    watched: std::collections::HashSet<PathBuf>,
    buffer: [u8; 16 * 1024],
}

#[cfg(target_os = "linux")]
impl DirectoryWatcher {
    /// Create the watcher and add watches for every directory under `dirs`
    /// (recursively). A missing directory is skipped.
    pub fn new(dirs: Vec<PathBuf>) -> std::io::Result<Self> {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut watcher = DirectoryWatcher {
            fd,
            dirs,
            watched: std::collections::HashSet::new(),
            buffer: [0u8; 16 * 1024],
        };
        watcher.add_watches()?;
        Ok(watcher)
    }

    /// Block forever, calling `on_change` after every debounced monitor event.
    pub fn run(&mut self, mut on_change: impl FnMut()) -> std::io::Result<()> {
        loop {
            let mut pollfd = libc::pollfd {
                fd: self.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pollfd, 1, -1) };
            if ready < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if ready == 0 {
                continue;
            }
            self.drain();
            // Coalesce the burst that a single install produces.
            std::thread::sleep(Duration::from_millis(DEBOUNCE_MS));
            self.drain();
            // A new subdirectory needs its own watch before the next event.
            let _ = self.add_watches();
            on_change();
        }
    }

    /// Read everything currently queued on the descriptor, ignoring EAGAIN.
    fn drain(&mut self) {
        loop {
            let count = unsafe {
                libc::read(
                    self.fd,
                    self.buffer.as_mut_ptr().cast::<libc::c_void>(),
                    self.buffer.len(),
                )
            };
            if count <= 0 {
                return;
            }
        }
    }

    /// Add a watch for every directory under `dirs`, plus each directory
    /// itself. Directories already watched are skipped; a later pass picks up
    /// new subdirectories.
    fn add_watches(&mut self) -> std::io::Result<()> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let mut stack: Vec<PathBuf> = self.dirs.clone();
        while let Some(dir) = stack.pop() {
            let Ok(metadata) = std::fs::metadata(&dir) else {
                continue;
            };
            if !metadata.is_dir() {
                continue;
            }
            if self.watched.insert(dir.clone()) {
                let Ok(c_path) = CString::new(dir.as_os_str().as_bytes()) else {
                    continue;
                };
                let wd = unsafe { libc::inotify_add_watch(self.fd, c_path.as_ptr(), WATCH_MASK) };
                if wd < 0 {
                    // A lost watch is not fatal: the index still refreshes on
                    // the next event, and a later pass retries.
                    self.watched.remove(&dir);
                }
            }
            if let Ok(read) = std::fs::read_dir(&dir) {
                for entry in read.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for DirectoryWatcher {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

/// Watch `dirs` (recursively) and call `on_change` after every debounced
/// monitor event. Blocks forever; run it on its own thread. A missing
/// directory is skipped, so a session with no `applications` folder simply
/// waits.
#[cfg(target_os = "linux")]
pub fn watch_dirs(dirs: Vec<PathBuf>, on_change: impl FnMut()) -> std::io::Result<()> {
    DirectoryWatcher::new(dirs)?.run(on_change)
}

/// Non-Linux builds have no inotify backend; the service still runs, it just
/// does not observe installs. (The shipping target is Linux.)
#[cfg(not(target_os = "linux"))]
pub fn watch_dirs(_dirs: Vec<PathBuf>, _on_change: impl FnMut()) -> std::io::Result<()> {
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "df-app-index-watch-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_created_desktop_file_wakes_the_watcher() {
        let dir = temp_dir("install");
        // Establish the watches before moving the watcher to its thread, so
        // the write below cannot race the first add_watch call.
        let mut watcher = DirectoryWatcher::new(vec![dir.clone()]).unwrap();

        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let runner = std::thread::spawn(move || {
            watcher
                .run(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
                .ok();
        });

        std::fs::write(
            Path::new(&dir).join("watched.desktop"),
            "[Desktop Entry]\nType=Application\nName=Watched\n",
        )
        .unwrap();

        // inotify is prompt; allow a wide window for a loaded CI host.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while hits.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(hits.load(Ordering::SeqCst) >= 1, "the watcher never fired");

        // The runner blocks forever; detach it and clean up the fixture.
        drop(runner);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
