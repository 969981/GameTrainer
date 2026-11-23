//! Read-only mGBA external adapter for Windows.
//! Linux CI only compiles the platform-neutral trainer-mgba core.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
