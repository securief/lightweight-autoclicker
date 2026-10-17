use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub const WINDOW_TITLE: &str = "Lightweight Autoclicker";

const LOCK_FILE_NAME: &str = "lightweight_autoclicker.lock";

const SHOW_SIGNAL_FILE: &str = "lightweight_autoclicker.show";

#[cfg(windows)]
const ERROR_SHARING_VIOLATION: i32 = 32;

pub enum Lock {
    Held { _file: File },
    Unprotected,
}

pub fn acquire() -> Option<Lock> {
    acquire_at(&lock_path(), &signal_path())
}

pub fn lock_path() -> PathBuf {
    std::env::temp_dir().join(LOCK_FILE_NAME)
}

pub fn signal_path() -> PathBuf {
    std::env::temp_dir().join(SHOW_SIGNAL_FILE)
}

pub fn take_show_signal() -> bool {
    take_show_signal_at(&signal_path())
}

fn take_show_signal_at(signal: &Path) -> bool {
    if !signal.exists() {
        return false;
    }
    std::fs::remove_file(signal).is_ok()
}

fn acquire_at(lock: &Path, signal: &Path) -> Option<Lock> {
    match try_lock(lock) {
        Ok(Some(_file)) => {
            let _ = std::fs::remove_file(signal);
            Some(Lock::Held { _file })
        }
        Ok(None) => {
            let _ = File::create(signal);
            None
        }
        Err(_) => Some(Lock::Unprotected),
    }
}

#[cfg(windows)]
fn try_lock(path: &Path) -> io::Result<Option<File>> {
    use std::os::windows::fs::OpenOptionsExt;

    let opened = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(path);

    match opened {
        Ok(file) => Ok(Some(file)),
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(unix)]
fn try_lock(path: &Path) -> io::Result<Option<File>> {
    use std::os::unix::io::AsRawFd;

    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;

    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        Ok(Some(file))
    } else {
        Ok(None)
    }
}

#[cfg(not(any(windows, unix)))]
fn try_lock(path: &Path) -> io::Result<Option<File>> {
    let _ = path;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lock(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lightweight_autoclicker_test_{name}_{}.lock",
            std::process::id()
        ))
    }

    fn test_signal(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lightweight_autoclicker_test_{name}_{}.show",
            std::process::id()
        ))
    }

    #[test]
    fn exclusive_lock_released_on_drop() {
        let path = test_lock("exclusive");
        let _ = std::fs::remove_file(&path);

        let first = try_lock(&path).expect("open first lock file");
        assert!(first.is_some(), "first instance must acquire the lock");

        let second = try_lock(&path).expect("open second lock file");
        assert!(second.is_none(), "second instance must be rejected");

        drop(first);
        let third = try_lock(&path).expect("open third lock file");
        assert!(
            third.is_some(),
            "lock must be released after the File is dropped"
        );

        drop(third);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fail_open_when_file_cannot_be_created() {
        let path = std::env::temp_dir()
            .join("lightweight_autoclicker_missing_dir")
            .join("lock");
        let signal = test_signal("failopen");
        let _ = std::fs::remove_file(&signal);
        assert!(
            matches!(acquire_at(&path, &signal), Some(Lock::Unprotected)),
            "a file open failure (not caused by another instance) must fail open"
        );
        assert!(
            !signal.exists(),
            "a failed lock open is no reason to write the signal (no first instance exists)"
        );
    }

    #[test]
    fn signal_written_by_second_instance_and_consumed_once() {
        let lock = test_lock("signal");
        let signal = test_signal("signal");
        let _ = std::fs::remove_file(&lock);
        let _ = std::fs::remove_file(&signal);

        let first = acquire_at(&lock, &signal).expect("first instance may run");
        assert!(matches!(first, Lock::Held { .. }));
        assert!(!signal.exists(), "a clean acquire leaves no signal behind");

        assert!(
            acquire_at(&lock, &signal).is_none(),
            "second instance must be rejected"
        );
        assert!(
            signal.exists(),
            "second instance must write the signal file"
        );

        assert!(take_show_signal_at(&signal), "signal must be readable");
        assert!(
            !take_show_signal_at(&signal),
            "signal must not be read twice"
        );
        assert!(
            !signal.exists(),
            "signal file must be removed once consumed"
        );

        drop(first);
        let _ = std::fs::remove_file(&lock);
    }

    #[test]
    fn stale_signal_cleared_on_acquire() {
        let lock = test_lock("stale");
        let signal = test_signal("stale");
        let _ = std::fs::remove_file(&lock);
        std::fs::write(&signal, b"").expect("write stale signal");

        let held = acquire_at(&lock, &signal).expect("first instance may run");
        assert!(matches!(held, Lock::Held { .. }));
        assert!(!signal.exists(), "stale signal must be removed");

        drop(held);
        let _ = std::fs::remove_file(&lock);
    }

    #[test]
    fn lock_and_signal_paths_live_in_temp_dir() {
        let lock = lock_path();
        assert!(lock.ends_with(LOCK_FILE_NAME));
        assert_eq!(lock.parent(), Some(std::env::temp_dir().as_path()));

        let signal = signal_path();
        assert!(signal.ends_with(SHOW_SIGNAL_FILE));
        assert_eq!(signal.parent(), Some(std::env::temp_dir().as_path()));
    }
}
