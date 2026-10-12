use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow,
};

const SW_RESTORE: i32 = 9;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Outcome of the single-instance check.
#[derive(Debug, PartialEq, Eq)]
pub enum SingleInstance {
    /// This process is the first instance and holds the named mutex for its
    /// lifetime (the handle is intentionally leaked, which is
    /// process-lifetime ownership).
    First,
    /// Another instance already exists; its window (located by title) has
    /// been restored and brought to the foreground. The caller must exit.
    AlreadyRunning,
    /// Mutex creation failed outright (e.g. the name collides with an
    /// existing non-mutex kernel object). The guard cannot work; the caller
    /// should report a fatal error instead of running unguarded.
    MutexFailed,
}

pub fn acquire_or_raise(mutex_name: &str, window_title: &str) -> SingleInstance {
    unsafe {
        let mutex = CreateMutexW(std::ptr::null(), 0, to_wide(mutex_name).as_ptr());
        if mutex.is_null() {
            return SingleInstance::MutexFailed;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let hwnd = FindWindowW(std::ptr::null(), to_wide(window_title).as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
            return SingleInstance::AlreadyRunning;
        }
        SingleInstance::First
    }
}
