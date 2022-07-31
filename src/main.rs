use log::LevelFilter;
use simple_logger::SimpleLogger;

mod utils;

#[cfg(target_os = "windows")]
mod conpty;

#[cfg(not(target_os = "windows"))]
mod xnix;
#[cfg(not(target_os = "windows"))]
use xnix::{bind , connect ,rbind ,rconnect };
#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
use win::{bind , connect ,rbind ,rconnect };

fn usage () {
	println!("Cliws - Lightweight interactive bind/reverse PTY shell");
	println!("https://github.com/b23r0/Cliws");
	println!("Usage: cliws [-p listen port] [-c ws address] [-l reverse port] [-r reverse addr] [command]");
}

fn main() {

	SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

	let arg_count = std::env::args().count();

	if  arg_count == 1{
		usage();
		return;
	}

	let first  = std::env::args().nth(1).expect("parameter not enough");

	match first.as_str() {
		"-l" => {

			let port = match std::env::args().nth(2) {
				None => {
					log::error!("not found listen port . eg : cliws -l 8000");
					return;
				},
				Some(p) => p
			};

			rbind(port);
			
		},
		"-r" => {
			let address = match std::env::args().nth(2) {
				None => {
					log::error!("not found reverse connection address . eg : cliws -r ws://127.0.0.1:8000 bash -i");
					return;
				},
				Some(p) => p
			};

			let subprocess = match std::env::args().nth(3) {
				None => {
					log::error!("not found command . eg : cliws -r ws://127.0.0.1:8000 bash -i");
					return;
				},
				Some(p) => p
			};

			let mut fullargs : Vec<String> = Vec::new();
			for i in 4..arg_count {
		
				let s = std::env::args().nth(i).expect("parse parameter faild");
				fullargs.push(s);
			}
			rconnect(address, subprocess, fullargs);
			
		},
		"-c" => {
			let connect_addr = match std::env::args().nth(2) {
				None => {
					log::error!("not found connection address . eg : cliws -c ws://127.0.0.1:8000");
					return;
				},
				Some(p) => p
			};
			connect(connect_addr);
			
		},
		"-p" => {
			let port = match std::env::args().nth(2) {
				None => {
					log::error!("not found listen port . eg : cliws -p 8000 bash -i");
					return;
				},
				Some(p) => p
			};
			let mut fullargs : Vec<String> = Vec::new();

			let subprocess = match std::env::args().nth(3) {
				None => {
					log::error!("not found command . eg : cliws -p 8000 bash -i");
					return;
				},
				Some(p) => p
			};
			
			for i in 4..arg_count {
		
				let s = std::env::args().nth(i).expect("parse parameter faild");
				fullargs.push(s);
			}
			bind(port, subprocess, fullargs);
			
		},

		_ => {
			usage();
			
		}
	}
}
