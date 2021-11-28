//! A library which provides an interface for [ConPTY].
//!
//! ```ignore
//! # // todo: determine why this test timeouts if runnin as a doc test but not as an example.
//! use std::io::prelude::*;
//!
//! fn main() {
//!     let proc = conpty::spawn("echo Hello World").unwrap();
//!     let mut reader = proc.output().unwrap();
//!
//!     println!("Process has pid={}", proc.pid());
//!
//!     proc.wait(None).unwrap();
//!
//!     let mut buf = [0; 1028];
//!     let n = reader.read(&mut buf).unwrap();
//!     assert!(String::from_utf8_lossy(&buf[..n]).contains("Hello World"));
//! }
//! ```
//!
//! [ConPTY]: https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/

#![allow(non_snake_case)]

pub mod console;
pub mod io;
pub mod util;

pub(crate) mod bindings {
    windows::include_bindings!();
}

use bindings::{
    Windows::Win32::Foundation::CloseHandle,
    Windows::Win32::Foundation::{HANDLE, PWSTR},
    Windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    Windows::Win32::System::Console::{
        ClosePseudoConsole, CreatePseudoConsole, GetConsoleMode, GetConsoleScreenBufferInfo,
        ResizePseudoConsole, SetConsoleMode, CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, COORD,
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, HPCON,
    },
    Windows::Win32::System::Pipes::CreatePipe,
    Windows::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess, GetProcessId,
        InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
        WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW, WAIT_TIMEOUT,
    },
    Windows::Win32::System::WindowsProgramming::INFINITE,
};

use std::collections::HashMap;
use std::{mem::size_of, ptr::null_mut};
use windows::HRESULT;

pub use windows::Error;

/// Spawns a command using `cmd.exe`.
pub fn spawn(cmd: impl Into<String>) -> windows::Result<Process> {
    Process::spawn(ProcAttr::cmd(cmd.into()))
}

/// The structure is resposible for interations with spawned process.
/// It handles IO and other operations related to a spawned process.
#[derive(Debug)]
pub struct Process {
    pty_input: HANDLE,
    pty_output: HANDLE,
    _proc: PROCESS_INFORMATION,
    _proc_info: STARTUPINFOEXW,
    _console: HPCON,
}

impl Process {
    fn spawn(attr: ProcAttr) -> windows::Result<Self> {
        enableVirtualTerminalSequenceProcessing()?;
        let (mut console, pty_reader, pty_writer) = createPseudoConsole()?;
        let startup_info = initializeStartupInfoAttachedToConPTY(&mut console)?;
        let proc = execProc(startup_info, attr)?;

        Ok(Self {
            pty_input: pty_writer,
            pty_output: pty_reader,
            _console: console,
            _proc: proc,
            _proc_info: startup_info,
        })
    }

    /// Resizes virtuall terminal.
    pub fn resize(&self, x: i16, y: i16) -> windows::Result<()> {
        unsafe { ResizePseudoConsole(self._console, COORD { X: x, Y: y }) }
    }

    /// Returns a process's pid.
    pub fn pid(&self) -> u32 {
        unsafe { GetProcessId(self._proc.hProcess) }
    }

    /// Termianates process with exit_code.
    pub fn exit(&self, code: u32) -> windows::Result<()> {
        unsafe { TerminateProcess(self._proc.hProcess, code).ok() }
    }

    /// Waits before process exists.
    pub fn wait(&self, timeout_millis: Option<u32>) -> windows::Result<u32> {
        unsafe {
            match timeout_millis {
                Some(timeout) => {
                    if WaitForSingleObject(self._proc.hProcess, timeout) == WAIT_TIMEOUT {
                        return Err(windows::Error::new(
                            HRESULT::from_thread(),
                            "Timeout is reached",
                        ));
                    }
                }
                None => {
                    WaitForSingleObject(self._proc.hProcess, INFINITE);
                }
            }

            let mut code = 0;
            GetExitCodeProcess(self._proc.hProcess, &mut code).ok()?;

            Ok(code)
        }
    }

    /// Is alive determines if a process is still running.
    ///
    /// IMPORTANT: Beware to use it in a way to stop reading when is_alive is false.
    //  Because at the point of calling method it may be alive but at the point of `read` call it may already not.
    pub fn is_alive(&self) -> bool {
        // https://stackoverflow.com/questions/1591342/c-how-to-determine-if-a-windows-process-is-running/5303889
        unsafe { WaitForSingleObject(self._proc.hProcess, 0) == WAIT_TIMEOUT }
    }

    /// Sets echo mode for a session.
    pub fn set_echo(&self, on: bool) -> windows::Result<()> {
        // todo: determine if this function is usefull and it works?
        let stdout_h = stdout_handle()?;
        unsafe {
            let mut mode = CONSOLE_MODE::default();
            GetConsoleMode(stdout_h, &mut mode).ok()?;

            match on {
                true => mode |= ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT,
                false => mode &= CONSOLE_MODE(!ENABLE_ECHO_INPUT.0),
            };

            SetConsoleMode(stdout_h, mode).ok()?;
            CloseHandle(stdout_h);
        }

        Ok(())
    }

    /// Returns a pipe writer to conPTY.
    pub fn input(&self) -> std::io::Result<io::PipeWriter> {
        // see [Self::output]
        util::clone_handle(self.pty_input).map(io::PipeWriter::new)
    }

    /// Returns a pipe reader from conPTY.
    pub fn output(&self) -> std::io::Result<io::PipeReader> {
        // It's crusial to clone first and not affect original HANDLE
        // as closing it closes all other's handles even though it's kindof unxpected.
        //
        // "
        // Closing a handle does not close the object.  It merely reduces the
        // "reference count".  When the reference count goes to zero, the object
        // itself is closed.  So, if you have a file handle, and you duplicate that
        // handle, the file now has two "references".  If you close one handle, the
        // file still has one reference, so the FILE cannot be closed.
        // "
        //
        // https://social.msdn.microsoft.com/Forums/windowsdesktop/en-US/1754715c-45b7-4d8c-ba56-a501ccaec12c/closehandle-amp-duplicatehandle?forum=windowsgeneraldevelopmentissues
        util::clone_handle(self.pty_output).map(io::PipeReader::new)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            ClosePseudoConsole(self._console);

            CloseHandle(self._proc.hProcess);
            CloseHandle(self._proc.hThread);

            DeleteProcThreadAttributeList(self._proc_info.lpAttributeList);
            let _ = Box::from_raw(self._proc_info.lpAttributeList.0 as _);

            CloseHandle(self.pty_input);
            CloseHandle(self.pty_output);
        }
    }
}

/// ProcAttr represents parameters for process to be spawned.
///
/// Interface is inspired by win32 `CreateProcess` function.
///
/// Generally to run a common process you can set commandline to a path to binary.
/// But if you're trying to spawn just a command in shell if must provide your shell first, like cmd.exe.
///
/// # Example
///
/// ```ignore
/// let attr = conpty::ProcAttr::default().commandline("pwsh").arg("echo", "world");
/// ```
#[derive(Default, Debug)]
pub struct ProcAttr {
    application: Option<String>,
    commandline: Option<String>,
    current_dir: Option<String>,
    args: Vec<String>,
    env: Option<HashMap<String, String>>,
}

impl ProcAttr {
    /// Runs a batch file in a default `CMD` interpretator.
    pub fn batch(file: impl AsRef<str>) -> Self {
        // To run a batch file, you must start the command interpreter; set lpApplicationName to cmd.exe and
        // set lpCommandLine to the following arguments: /c plus the name of the batch file.
        //
        // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
        let inter = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd".to_string());
        let args = format!("/C {:?}", file.as_ref());

        Self::default().application(inter).commandline(args)
    }

    /// Runs a command from `cmd.exe`
    pub fn cmd(commandline: impl AsRef<str>) -> Self {
        let args = format!("cmd /C {}", commandline.as_ref());

        Self::default().commandline(args)
    }

    /// Sets commandline argument.
    pub fn commandline(mut self, cmd: impl Into<String>) -> Self {
        self.commandline = Some(cmd.into());
        self
    }

    /// Sets application argument.
    /// Must be a path to a binary.
    pub fn application(mut self, application: impl Into<String>) -> Self {
        self.application = Some(application.into());
        self
    }

    /// Sets current dir.
    pub fn current_dir(mut self, dir: impl Into<String>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Sets a list of arguments as process arguments.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Adds an argument to a list of process arguments.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Sets a list of env variables as process env variables.
    ///
    /// If envs isn't set they will be inhirited from parent process.
    pub fn envs(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    /// Adds an env variable to process env variables list.
    ///
    /// If any envs isn't added the environment list will be inhirited from parent process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self.env {
            Some(env) => {
                env.insert(key.into(), value.into());
                self
            }
            None => self.envs(HashMap::new()).env(key.into(), value.into()),
        }
    }

    /// Spawns a process with set attributes.
    pub fn spawn(self) -> windows::Result<Process> {
        Process::spawn(self)
    }
}

fn enableVirtualTerminalSequenceProcessing() -> windows::Result<()> {
    let stdout_h = stdout_handle()?;
    unsafe {
        let mut mode = CONSOLE_MODE::default();
        GetConsoleMode(stdout_h, &mut mode).ok()?;
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING; // DISABLE_NEWLINE_AUTO_RETURN
        SetConsoleMode(stdout_h, mode).ok()?;

        CloseHandle(stdout_h);
    }

    Ok(())
}

fn createPseudoConsole() -> windows::Result<(HPCON, HANDLE, HANDLE)> {
    let (pty_in, con_writer) = pipe()?;
    let (con_reader, pty_out) = pipe()?;

    let size = inhirentConsoleSize()?;

    let console = unsafe { CreatePseudoConsole(size, pty_in, pty_out, 0)? };

    // Note: We can close the handles to the PTY-end of the pipes here
    // because the handles are dup'ed into the ConHost and will be released
    // when the ConPTY is destroyed.
    unsafe {
        CloseHandle(pty_in);
    }
    unsafe {
        CloseHandle(pty_out);
    }

    Ok((console, con_reader, con_writer))
}

fn inhirentConsoleSize() -> windows::Result<COORD> {
    let stdout_h = stdout_handle()?;
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    unsafe {
        GetConsoleScreenBufferInfo(stdout_h, &mut info).ok()?;
        CloseHandle(stdout_h);
    };

    let mut size = COORD { X: 24, Y: 80 };
    size.X = info.srWindow.Right - info.srWindow.Left + 1;
    size.Y = info.srWindow.Bottom - info.srWindow.Top + 1;

    Ok(size)
}

// const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 22 | 0x0002_0000;
const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x00020016;

fn initializeStartupInfoAttachedToConPTY(hPC: &mut HPCON) -> windows::Result<STARTUPINFOEXW> {
    let mut siEx = STARTUPINFOEXW::default();
    siEx.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;

    let mut size: usize = 0;
    let res = unsafe {
        InitializeProcThreadAttributeList(LPPROC_THREAD_ATTRIBUTE_LIST::default(), 1, 0, &mut size)
    };
    if res.as_bool() || size == 0 {
        return Err(windows::Error::new(HRESULT::from_thread(), ""));
    }

    // SAFETY
    // we leak the memory intentionally,
    // it will be freed on DROP.
    let lpAttributeList = vec![0u8; size].into_boxed_slice();
    let lpAttributeList = Box::leak(lpAttributeList);

    siEx.lpAttributeList = LPPROC_THREAD_ATTRIBUTE_LIST(lpAttributeList.as_mut_ptr().cast());

    unsafe {
        InitializeProcThreadAttributeList(siEx.lpAttributeList, 1, 0, &mut size).ok()?;
        UpdateProcThreadAttribute(
            siEx.lpAttributeList,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            hPC.0 as _,
            size_of::<HPCON>(),
            null_mut(),
            null_mut(),
        )
        .ok()?;
    }

    Ok(siEx)
}

fn execProc(
    mut startup_info: STARTUPINFOEXW,
    attr: ProcAttr,
) -> windows::Result<PROCESS_INFORMATION> {
    if attr.commandline.is_none() && attr.application.is_none() {
        panic!("")
    }

    let mut commandline = pwstr_param(attr.commandline);
    let mut application = pwstr_param(attr.application);
    let mut current_dir = pwstr_param(attr.current_dir);
    let env = match attr.env {
        Some(env) => Box::<[u16]>::into_raw(environment_block_unicode(env).into_boxed_slice()) as _,
        None => null_mut(),
    };

    let mut proc_info = PROCESS_INFORMATION::default();
    let result = unsafe {
        CreateProcessW(
            application.abi(),
            commandline.abi(),
            null_mut(),
            null_mut(),
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT, // CREATE_UNICODE_ENVIRONMENT | CREATE_NEW_CONSOLE
            env,
            current_dir.abi(),
            &mut startup_info.StartupInfo,
            &mut proc_info,
        )
        .ok()
    };

    if !env.is_null() {
        unsafe {
            ::std::boxed::Box::from_raw(env);
        }
    }

    result?;

    Ok(proc_info)
}

fn pipe() -> windows::Result<(HANDLE, HANDLE)> {
    let mut p_in = HANDLE::default();
    let mut p_out = HANDLE::default();
    unsafe { CreatePipe(&mut p_in, &mut p_out, std::ptr::null_mut(), 0).ok()? };

    Ok((p_in, p_out))
}

fn stdout_handle() -> windows::Result<HANDLE> {
    // we can't use `GetStdHandle(STD_OUTPUT_HANDLE)`
    // because it doesn't work when the IO is redirected
    //
    // https://stackoverflow.com/questions/33476316/win32-getconsolemode-error-code-6

    let hConsole = unsafe {
        CreateFileW(
            "CONOUT$",
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            HANDLE::NULL,
        )
    };

    if hConsole.is_null() || hConsole.is_invalid() {
        Err(HRESULT::from_thread().into())
    } else {
        Ok(hConsole)
    }
}

fn environment_block_unicode(env: HashMap<String, String>) -> Vec<u16> {
    if env.is_empty() {
        // two '\0' in UTF-16/UCS-2
        // four '\0' in UTF-8
        return vec![0, 0];
    }

    let mut b = Vec::new();
    for (key, value) in env {
        let part = format!("{}={}\0", key, value);
        b.extend(part.encode_utf16());
    }

    b.push(0);

    b
}

// if given string is empty there will be produced a "\0" string in UTF-16
fn pwstr_param(s: Option<String>) -> windows::Param<'static, PWSTR> {
    use windows::IntoParam;
    match s {
        Some(s) => {
            // https://github.com/microsoft/windows-rs/blob/ba61866b51bafac94844a242f971739583ffa70e/crates/gen/src/pwstr.rs
            s.into_param()
        }
        None => {
            // the memory will be zeroed
            // https://github.com/microsoft/windows-rs/blob/e1ab47c00b10b220d1372e4cdbe9a689d6365001/src/runtime/param.rs
            windows::Param::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;
    use std::iter::FromIterator;

    #[test]
    pub fn close_one_pty_input_doesnt_close_others() {
        let proc = spawn("cmd").unwrap();
        let writer1 = proc.input().unwrap();
        let mut writer2 = proc.input().unwrap();

        assert!(writer2.write(b"").is_ok());

        drop(writer1);

        assert!(writer2.write(b"").is_ok());
    }

    #[test]
    pub fn non_blocking_read() {
        let proc = spawn("cmd").unwrap();
        let mut reader = proc.output().unwrap();
        reader.set_non_blocking_mode().unwrap();

        let mut buf = [0; 1028];
        loop {
            match reader.read(&mut buf) {
                Ok(_) => break,
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(err) => Err(err).unwrap(),
            }
        }
    }

    #[test]
    pub fn non_blocking_mode_affects_all_readers() {
        let proc = spawn("cmd").unwrap();
        let mut reader1 = proc.output().unwrap();
        let mut reader2 = proc.output().unwrap();
        reader2.set_non_blocking_mode().unwrap();

        assert_eq!(
            reader1.read(&mut [0; 128]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[test]
    pub fn dropping_one_reader_doesnt_affect_others() {
        let proc = spawn("cmd").unwrap();
        let mut reader1 = proc.output().unwrap();
        let reader2 = proc.output().unwrap();

        drop(reader2);

        reader1.set_non_blocking_mode().unwrap();
        assert_eq!(
            reader1.read(&mut [0; 128]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    // not sure if's desired behaiviour
    // todo: timeout for wait/exit
    #[test]
    pub fn env_parameter() {
        let batch = r#"if "%TEST_ENV%"=="123456" (exit 0) else (exit 1)"#;
        let proc = ProcAttr::cmd(batch.to_string())
            .env("TEST_ENV".to_string(), "123456".to_string())
            .spawn()
            .unwrap();
        assert_eq!(proc.wait(None).unwrap(), 0);

        let proc = ProcAttr::cmd(batch.to_string())
            .env("TEST_ENV".to_string(), "NOT_CORRENT_VALUE".to_string())
            .spawn()
            .unwrap();
        assert_eq!(proc.wait(None).unwrap(), 1);

        // not set
        let proc = ProcAttr::cmd(batch.to_string()).spawn().unwrap();
        assert_eq!(proc.wait(None).unwrap(), 1);
    }

    #[test]
    fn env_block_test() {
        assert_eq!(
            environment_block_unicode(HashMap::from_iter([("asd".to_string(), "qwe".to_string())])),
            str_to_utf16("asd=qwe\0\0")
        );
        assert!(matches!(environment_block_unicode(HashMap::from_iter([
                ("asd".to_string(), "qwe".to_string()),
                ("zxc".to_string(), "123".to_string())
            ])), s if s == str_to_utf16("asd=qwe\0zxc=123\0\0") || s == str_to_utf16("zxc=123\0asd=qwe\0\0")));
        assert_eq!(
            environment_block_unicode(HashMap::from_iter([])),
            str_to_utf16("\0\0")
        );
    }

    fn str_to_utf16(s: impl AsRef<str>) -> Vec<u16> {
        s.as_ref().encode_utf16().collect()
    }
}
