//! Small shared helpers.

/// Windows `CREATE_NO_WINDOW` flag — keeps spawned console tools (steamcmd, taskkill,
/// tasklist) from flashing a black console window while the GUI is running.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Windows `CREATE_NEW_CONSOLE` flag — gives the launched process its own console.
/// The Palworld console server build (`-Cmd`) is unstable without a real console
/// (it crashes with "SECURE CRT: Invalid parameter detected"), so we launch the
/// server this way — matching how double-clicking `PalServer.exe` behaves.
#[cfg(windows)]
pub const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Helpers to control the console behaviour of a spawned `Command` on Windows.
/// On other platforms these are no-ops so the rest of the code stays clean.
pub trait CommandExt {
    /// Suppress any console window (for short-lived helper tools).
    fn hidden(&mut self) -> &mut Self;
    /// Give the process its own new console window (for the game server).
    fn new_console(&mut self) -> &mut Self;
}

impl CommandExt for std::process::Command {
    #[cfg(windows)]
    fn hidden(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt as _;
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hidden(&mut self) -> &mut Self {
        self
    }

    #[cfg(windows)]
    fn new_console(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt as _;
        self.creation_flags(CREATE_NEW_CONSOLE)
    }

    #[cfg(not(windows))]
    fn new_console(&mut self) -> &mut Self {
        self
    }
}
