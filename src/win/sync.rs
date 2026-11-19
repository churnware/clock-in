//! Named kernel objects: a mutex that marks the daemon as running, and an
//! event other processes set to ask it to stop.

use std::{process::Command, time::Duration};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0},
    System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, CreateEventW, CreateMutexW, DETACHED_PROCESS,
        OpenEventW, OpenMutexW, SetEvent, WaitForSingleObject,
    },
};

use super::wide;

const MUTEX: &str = r"Local\clock-in-daemon";
const STOP_EVENT: &str = r"Local\clock-in-stop";
const SYNCHRONIZE: u32 = 0x0010_0000;
const EVENT_MODIFY_STATE: u32 = 0x0002;

pub struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: we own the handle.
        unsafe { CloseHandle(self.0) };
    }
}

/// Claim the single-instance mutex. `None` if a daemon already holds it.
pub fn claim_daemon() -> Option<Handle> {
    let name = wide(MUTEX);
    // SAFETY: valid NUL-terminated name; the handle is owned by `Handle`.
    unsafe {
        let h = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if h.is_null() {
            return None;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(h);
            return None;
        }
        Some(Handle(h))
    }
}

pub fn daemon_running() -> bool {
    let name = wide(MUTEX);
    // SAFETY: valid NUL-terminated name; the handle is closed immediately.
    unsafe {
        let h = OpenMutexW(SYNCHRONIZE, 0, name.as_ptr());
        if h.is_null() {
            return false;
        }
        CloseHandle(h);
        true
    }
}

pub struct StopEvent(Handle);

impl StopEvent {
    pub fn create() -> Option<Self> {
        let name = wide(STOP_EVENT);
        // SAFETY: valid name; manual-reset, initially unset.
        let h = unsafe { CreateEventW(std::ptr::null(), 1, 0, name.as_ptr()) };
        (!h.is_null()).then(|| StopEvent(Handle(h)))
    }

    /// Sleep for `timeout`; returns `true` early if a stop was requested.
    pub fn wait(&self, timeout: Duration) -> bool {
        // SAFETY: the event handle is valid for the lifetime of `self`.
        unsafe { WaitForSingleObject(self.0.0, timeout.as_millis() as u32) == WAIT_OBJECT_0 }
    }
}

/// Ask a running daemon to stop. Returns `false` if none is listening.
pub fn request_stop() -> bool {
    let name = wide(STOP_EVENT);
    // SAFETY: valid name; the handle is closed immediately.
    unsafe {
        let h = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if h.is_null() {
            return false;
        }
        SetEvent(h);
        CloseHandle(h);
        true
    }
}

/// Run `clock-in daemon` detached from this console, with no window.
pub fn spawn_detached() -> std::io::Result<u32> {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn()?;
    Ok(child.id())
}
