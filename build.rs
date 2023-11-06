use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let hosturl = match env::var("HOSTURL") {
        Ok(key) => key,
        Err(error) => panic!("Error: {:?} ADDR ENV variable required", error),
    };

    let port = match env::var("PORT") {
        Ok(key) => key,
        Err(error) => panic!("Error: {:?} PORT ENV variable required", error),
    };

    let bind_shell = match env::var("BIND") {
        Ok(b) => b,
        Err(_) => "0".to_string(),
    };

    let dest_addr = Path::new(&out_dir).join("hosturl");
    let dest_port = Path::new(&out_dir).join("port");
    let dest_bindshell = Path::new(&out_dir).join("bind");

    let mut f: File = File::create(dest_addr).unwrap();
    let mut h: File = File::create(dest_bindshell).unwrap();
    let mut d: File = File::create(dest_port).unwrap();

    match f.write(hosturl.as_bytes()) {
        Ok(_) => {},
        Err(error) => panic!("Unable to write addr variable to build folder: {:?}", error),
    };

    match h.write(bind_shell.as_bytes()) {
        Ok(_) => {},
        Err(error) => panic!("Unable to write bind shell option to build folder: {:?}", error),
    };

    match d.write(port.as_bytes()) {
        Ok(_) => {},
        Err(error) => panic!("Unable to write port variable to build folder: {:?}", error),
    };
}