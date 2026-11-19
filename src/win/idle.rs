//! How long the user has been idle, and a harmless nudge to reset it.

use std::{mem::size_of, time::Duration};
use windows_sys::Win32::{
    System::SystemInformation::GetTickCount,
    UI::Input::KeyboardAndMouse::{
        GetLastInputInfo, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        LASTINPUTINFO, SendInput, VK_F15,
    },
};

/// Time since the last keyboard or mouse input, system-wide.
pub fn idle_for() -> Duration {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` is a valid, correctly sized LASTINPUTINFO.
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return Duration::ZERO;
    }
    let now = unsafe { GetTickCount() };
    Duration::from_millis(now.wrapping_sub(info.dwTime) as u64)
}

/// Press and release F15, a key no keyboard has and no app binds.
/// This resets the system idle timer that Teams watches.
pub fn nudge() -> bool {
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_F15,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [key(0), key(KEYEVENTF_KEYUP)];
    // SAFETY: `inputs` is a valid array of INPUT structs of the stated size.
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    sent == inputs.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends a real F15 to the desktop, so it's opt-in: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn nudge_resets_idle() {
        std::thread::sleep(Duration::from_millis(1500));
        assert!(nudge());
        assert!(idle_for() < Duration::from_millis(500));
    }
}
