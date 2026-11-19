//! Thin wrappers over the Win32 calls clock-in needs.

pub mod idle;
pub mod sync;
pub mod teams;

/// UTF-16, NUL-terminated, for `*W` APIs.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
