//! Small shared helpers.

/// Windows `CREATE_NO_WINDOW` flag — keeps spawned console tools (steamcmd, taskkill,
/// tasklist) from flashing a black console window while the GUI is running.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Extension used to attach the no-window creation flag to a `Command` on Windows.
/// On other platforms this is a no-op so the rest of the code stays clean.
pub trait CommandExt {
    fn hidden(&mut self) -> &mut Self;
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
}
