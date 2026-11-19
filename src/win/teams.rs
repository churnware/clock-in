//! Find, close and launch the new Microsoft Teams client (`ms-teams.exe`).

use std::{mem::size_of, process::Command, thread, time::Duration};
use windows_sys::Win32::{
    Foundation::{BOOL, CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess},
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
    },
};

const EXE: &str = "ms-teams.exe";
const APP_ID: &str = r"shell:AppsFolder\MSTeams_8wekyb3d8bbwe!MSTeams";

/// Process ids of every running `ms-teams.exe`.
pub fn pids() -> Vec<u32> {
    let mut out = Vec::new();
    // SAFETY: standard Toolhelp32 enumeration; the snapshot handle is closed below.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case(EXE) {
                out.push(entry.th32ProcessID);
            }
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
    }
    out
}

pub fn is_running() -> bool {
    !pids().is_empty()
}

/// Quit Teams: ask its windows to close, then end whatever is left.
/// Teams minimises to the tray on close, so the second step does the real work.
pub fn quit() -> usize {
    let pids = pids();
    if pids.is_empty() {
        return 0;
    }
    post_close(&pids);
    thread::sleep(Duration::from_secs(2));
    let remaining = self::pids();
    for pid in &remaining {
        // SAFETY: the handle is checked and closed; terminating our own user's process.
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, 0, *pid);
            if !h.is_null() {
                TerminateProcess(h, 0);
                CloseHandle(h);
            }
        }
    }
    pids.len()
}

fn post_close(pids: &[u32]) {
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the `&[u32]` passed to EnumWindows below, alive for the call.
        let pids = unsafe { &*(lparam as *const &[u32]) };
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pids.contains(&pid) && IsWindowVisible(hwnd) != 0 {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
        1
    }
    // SAFETY: the callback only reads `pids` during this synchronous call.
    unsafe { EnumWindows(Some(each), &pids as *const &[u32] as LPARAM) };
}

/// Start Teams through its packaged app id.
pub fn launch() -> std::io::Result<()> {
    Command::new("explorer.exe").arg(APP_ID).spawn().map(|_| ())
}
