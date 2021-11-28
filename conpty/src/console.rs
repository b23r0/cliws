use crate::bindings::{
    Windows::Win32::Foundation::HANDLE,
    Windows::Win32::System::Console::{
        GetConsoleMode, SetConsoleMode, CONSOLE_MODE, DISABLE_NEWLINE_AUTO_RETURN,
        ENABLE_ECHO_INPUT, ENABLE_EXTENDED_FLAGS, ENABLE_INSERT_MODE, ENABLE_LINE_INPUT,
        ENABLE_MOUSE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_QUICK_EDIT_MODE,
        ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_WINDOW_INPUT,
    },
    Windows::Win32::System::Threading::{WaitForSingleObject, WAIT_OBJECT_0},
};

/// Console doesn't owns handles.
/// So you need to manage there lifetime on there own.
pub struct Console {
    stdin: HANDLE,
    stdout: HANDLE,
    stderr: HANDLE,
    stdin_mode: CONSOLE_MODE,
    stdout_mode: CONSOLE_MODE,
    stderr_mode: CONSOLE_MODE,
}

impl Console {
    pub fn current() -> windows::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        let stdin = HANDLE(std::io::stdin().as_raw_handle() as isize);
        let stdout = HANDLE(std::io::stdout().as_raw_handle() as isize);
        let stderr = HANDLE(std::io::stderr().as_raw_handle() as isize);

        let stdin_mode = get_console_mode(stdin)?;
        let stdout_mode = get_console_mode(stdout)?;
        let stderr_mode = get_console_mode(stderr)?;

        Ok(Self {
            stderr,
            stderr_mode,
            stdin,
            stdin_mode,
            stdout,
            stdout_mode,
        })
    }

    pub fn set_raw(&self) -> windows::Result<()> {
        set_raw_stdin(self.stdin, self.stdin_mode)?;

        unsafe {
            SetConsoleMode(self.stdout, self.stdout_mode | DISABLE_NEWLINE_AUTO_RETURN).ok()?;
        }
        unsafe {
            SetConsoleMode(self.stderr, self.stderr_mode | DISABLE_NEWLINE_AUTO_RETURN).ok()?;
        }

        Ok(())
    }

    pub fn reset(&self) -> windows::Result<()> {
        for (handle, mode) in self.streams() {
            unsafe { SetConsoleMode(handle, mode).ok()? };
        }

        Ok(())
    }

    pub fn is_stdin_not_empty(&self) -> std::io::Result<bool> {
        // https://stackoverflow.com/questions/23164492/how-can-i-detect-if-there-is-input-waiting-on-stdin-on-windows
        Ok(unsafe { WaitForSingleObject(self.stdin, 0) == WAIT_OBJECT_0 })
    }

    fn streams(&self) -> [(HANDLE, CONSOLE_MODE); 3] {
        [
            (self.stdin, self.stdin_mode),
            (self.stdout, self.stdout_mode),
            (self.stderr, self.stderr_mode),
        ]
    }
}

fn get_console_mode(h: HANDLE) -> windows::Result<CONSOLE_MODE> {
    let mut mode = CONSOLE_MODE::default();
    unsafe {
        GetConsoleMode(h, &mut mode).ok()?;
    }
    Ok(mode)
}

fn set_raw_stdin(stdin: HANDLE, mut mode: CONSOLE_MODE) -> windows::Result<()> {
    mode &= CONSOLE_MODE(!ENABLE_ECHO_INPUT.0);
    mode &= CONSOLE_MODE(!ENABLE_LINE_INPUT.0);
    mode &= CONSOLE_MODE(!ENABLE_MOUSE_INPUT.0);
    mode &= CONSOLE_MODE(!ENABLE_WINDOW_INPUT.0);
    mode &= CONSOLE_MODE(!ENABLE_PROCESSED_INPUT.0);

    mode |= CONSOLE_MODE(ENABLE_EXTENDED_FLAGS);
    mode |= CONSOLE_MODE(ENABLE_INSERT_MODE.0);
    mode |= CONSOLE_MODE(ENABLE_QUICK_EDIT_MODE.0);

    let vtInputSupported = true;
    if vtInputSupported {
        mode |= ENABLE_VIRTUAL_TERMINAL_INPUT;
    }

    unsafe {
        SetConsoleMode(stdin, mode).ok()?;
    }

    Ok(())
}
