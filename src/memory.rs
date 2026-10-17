#[cfg(all(unix, target_pointer_width = "64"))]
use std::ffi::c_void;

pub const BG_THREAD_STACK: usize = 64 * 1024;

#[cfg(windows)]
pub fn trim_memory() {
    unsafe {
        let proc = windows_sys::Win32::System::Threading::GetCurrentProcess();
        windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet(proc);
    }
}

#[cfg(target_os = "linux")]
pub fn trim_memory() {
    unsafe extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    unsafe {
        let _ = malloc_trim(0);
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn trim_memory() {}

#[cfg(windows)]
pub fn map_file_readonly(path: &str) -> Option<&'static [u8]> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, MapViewOfFile, FILE_MAP_READ, PAGE_READONLY,
    };

    let len = usize::try_from(std::fs::metadata(path).ok()?.len()).ok()?;
    if len == 0 {
        return None;
    }

    let file = std::fs::File::open(path).ok()?;
    unsafe {
        let mapping = CreateFileMappingW(
            file.as_raw_handle() as HANDLE,
            std::ptr::null(),
            PAGE_READONLY,
            0,
            0,
            std::ptr::null(),
        );
        if mapping.is_null() {
            return None;
        }
        let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
        if view.Value.is_null() {
            CloseHandle(mapping);
            return None;
        }
        let _ = mapping;
        Some(std::slice::from_raw_parts(view.Value as *const u8, len))
    }
}

#[cfg(all(unix, target_pointer_width = "64"))]
pub fn map_file_readonly(path: &str) -> Option<&'static [u8]> {
    use std::os::fd::AsRawFd;

    const PROT_READ: i32 = 1;
    const MAP_PRIVATE: i32 = 2;
    const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;

    unsafe extern "C" {
        fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut c_void;
    }

    let len = usize::try_from(std::fs::metadata(path).ok()?.len()).ok()?;
    if len == 0 {
        return None;
    }

    let file = std::fs::File::open(path).ok()?;
    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ,
            MAP_PRIVATE,
            file.as_raw_fd(),
            0,
        )
    };
    if ptr == MAP_FAILED || ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) })
}

#[cfg(not(any(windows, all(unix, target_pointer_width = "64"))))]
pub fn map_file_readonly(_path: &str) -> Option<&'static [u8]> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP_SUPPORTED: bool = cfg!(windows) || cfg!(all(unix, target_pointer_width = "64"));

    fn test_file(name: &str, data: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "lightweight_autoclicker_map_{name}_{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, data).expect("write test file");
        path
    }

    #[test]
    fn map_file_readonly_contents_match_file() {
        if !MAP_SUPPORTED {
            return;
        }
        let data: Vec<u8> = (0..(4096 * 2 + 123)).map(|i| (i % 251) as u8).collect();
        let path = test_file("contents", &data);
        let mapped = map_file_readonly(path.to_str().unwrap()).expect("mapping must succeed");

        assert_eq!(
            mapped.len(),
            data.len(),
            "view length must equal file length"
        );
        assert_eq!(mapped, data.as_slice(), "view contents must match the file");
        assert_eq!(
            mapped[mapped.len() - 1],
            data[data.len() - 1],
            "last byte must be readable (page boundary)"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn map_file_readonly_rejects_empty_file() {
        if !MAP_SUPPORTED {
            return;
        }
        let path = test_file("empty", b"");
        assert!(
            map_file_readonly(path.to_str().unwrap()).is_none(),
            "a 0-byte file must return None"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn map_file_readonly_missing_path_returns_none() {
        let path = std::env::temp_dir()
            .join("lightweight_autoclicker_missing_dir")
            .join("font.ttf");
        assert!(map_file_readonly(path.to_str().unwrap()).is_none());
    }

    #[test]
    fn map_file_readonly_system_font_candidates_are_valid() {
        if !MAP_SUPPORTED {
            return;
        }
        for candidate in [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ] {
            if !std::path::Path::new(candidate).exists() {
                continue;
            }
            let mapped = map_file_readonly(candidate).expect("system font must be mappable");
            assert!(mapped.len() > 10_000, "font {candidate} is too small");
            let head = &mapped[..4];
            let ok = head == [0x00, 0x01, 0x00, 0x00].as_slice()
                || head == b"OTTO".as_slice()
                || head == b"true".as_slice();
            assert!(ok, "unrecognized font header for {candidate}: {head:?}");
        }
    }
}
