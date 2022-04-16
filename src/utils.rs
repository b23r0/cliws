#[cfg(target_os = "windows")]
use windows::Win32::{System::Console::{GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE}, Foundation::HANDLE};

pub static MAGIC_FLAG : [u8;2] = [0x37, 0x37];

pub fn makeword(a : u8, b : u8) -> u16{
    ((a as u16) << 8) | b as u16
}

pub fn splitword(a : u16) -> (u8 , u8) {
    ((a >> 8) as u8  , a as u8)
}

#[cfg(target_os = "windows")]
pub fn get_stdout_handle() -> HANDLE{
    unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }
}

#[cfg(target_os = "windows")]
pub fn get_stdin_handle() -> HANDLE{
    unsafe { GetStdHandle(STD_INPUT_HANDLE) }
}

#[cfg(target_os = "windows")]
pub fn handle_to_rawhandle(h : &HANDLE) -> std::os::windows::prelude::RawHandle {
    std::os::windows::prelude::RawHandle::from(h.0 as *mut std::ffi::c_void)
}