use crate::bindings::{
    Windows::Win32::Foundation::{CloseHandle, HANDLE},
    Windows::Win32::Storage::FileSystem::{FlushFileBuffers, ReadFile, WriteFile},
    Windows::Win32::System::Pipes::{SetNamedPipeHandleState, PIPE_NOWAIT},
    Windows::Win32::System::WindowsProgramming::PIPE_WAIT,
};

use crate::util::{clone_handle, win_error_to_io};
use std::io::{self, Read, Write};
use std::ptr::null_mut;
use windows::HRESULT;

/// PipeReader wraps a win32 pipe to provide a [std::io::Read] interface.
/// It also provides a non_blocking mode settings.
#[derive(Debug)]
pub struct PipeReader {
    handle: HANDLE,
}

impl PipeReader {
    /// Returns a new instance of PipeReader.
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Sets a pipe to a non blocking mode.
    ///
    /// IMPORTANT: It affects all dupped descriptors (All cloned `HANDLE`s, `FILE`s, `PipeReader`s)
    ///
    /// Mainly developed to not pile down libraries to include any windows API crate.
    pub fn set_non_blocking_mode(&mut self) -> io::Result<()> {
        let mut nowait = PIPE_NOWAIT;
        unsafe {
            SetNamedPipeHandleState(self.handle, &mut nowait.0, null_mut(), null_mut())
                .ok()
                .map_err(win_error_to_io)?;
        }
        Ok(())
    }

    /// Sets a pipe to a blocking mode.
    ///
    /// IMPORTANT: It affects all dupped descriptors (All cloned `HANDLE`s, `FILE`s, `PipeReader`s)
    ///
    /// Mainly developed to not pile down libraries to include any windows API crate.
    pub fn set_blocking_mode(&mut self) -> io::Result<()> {
        let mut nowait = PIPE_WAIT;
        unsafe {
            SetNamedPipeHandleState(self.handle, &mut nowait, null_mut(), null_mut())
                .ok()
                .map_err(win_error_to_io)?;
        }
        Ok(())
    }

    /// Tries to clone a instance to a new one.
    /// All cloned instances share the same underlaying data so
    /// Reading from one cloned pipe will affect an original pipe.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        clone_handle(self.handle).map(Self::new)
    }
}

impl Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut n = 0;
        let buf_size = buf.len() as u32;

        match unsafe {
            ReadFile(
                self.handle,
                buf.as_mut_ptr() as _,
                buf_size,
                &mut n,
                null_mut(),
            )
            .ok()
        } {
            Ok(()) => Ok(n as usize),
            // https://stackoverflow.com/questions/34504970/non-blocking-read-on-os-pipe-on-windows
            Err(err) if err.code() == HRESULT::from_win32(232) => {
                Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, err))
            }
            Err(err) => Err(win_error_to_io(err)),
        }
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl Into<std::fs::File> for PipeReader {
    fn into(self) -> std::fs::File {
        use std::os::windows::io::FromRawHandle;
        unsafe { std::fs::File::from_raw_handle(self.handle.0 as _) }
    }
}

/// PipeWriter implements [std::io::Write] interface for win32 pipe.
#[derive(Debug)]
pub struct PipeWriter {
    handle: HANDLE,
}

impl PipeWriter {
    /// Creates a new instance of PipeWriter.
    ///
    /// It owns a HANDLE.
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Tries to make a clone of PipeWriter.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        clone_handle(self.handle).map(Self::new)
    }
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut n = 0;
        let buf_size = buf.len() as u32;

        unsafe {
            WriteFile(self.handle, buf.as_ptr() as _, buf_size, &mut n, null_mut())
                .ok()
                .map_err(win_error_to_io)?;
        }

        Ok(n as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unsafe {
            FlushFileBuffers(self.handle)
                .ok()
                .map_err(win_error_to_io)?;
        }
        Ok(())
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle).ok().unwrap();
        }
    }
}

impl Into<std::fs::File> for PipeWriter {
    fn into(self) -> std::fs::File {
        use std::os::windows::io::FromRawHandle;
        unsafe { std::fs::File::from_raw_handle(self.handle.0 as _) }
    }
}
