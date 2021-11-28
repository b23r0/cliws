use crate::bindings::{
    Windows::Win32::Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE},
    Windows::Win32::System::Threading::{GetCurrentProcess, WaitForSingleObject, WAIT_OBJECT_0},
};

use std::io;

/// clone_handle can be used to clone a general HANDLE.
pub fn clone_handle(handle: HANDLE) -> std::io::Result<HANDLE> {
    let mut cloned_handle = HANDLE::default();
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut cloned_handle,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
        .ok()
        .map_err(win_error_to_io)?;
    }

    Ok(cloned_handle)
}

/// is_handle_ready can be used with FILE HANDLEs to determine if there's
/// something to read.
pub fn is_handle_ready(handle: HANDLE) -> std::io::Result<bool> {
    Ok(unsafe { WaitForSingleObject(handle, 0) == WAIT_OBJECT_0 })
}

pub(crate) fn win_error_to_io(err: windows::Error) -> io::Error {
    let code = err.code();
    io::Error::from_raw_os_error(code.0 as i32)
}
