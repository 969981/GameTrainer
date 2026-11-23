//! Read-only mGBA external adapter for Windows.
//! Linux CI only compiles the platform-neutral trainer-mgba core.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(windows)]
mod m8_win;
#[cfg(windows)]
pub use m8_win::read_desmume_runtime_rom_identity;
