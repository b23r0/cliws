use log::LevelFilter;
use simple_logger::SimpleLogger;

#[cfg(target_os = "linux")]
mod xnix;
#[cfg(target_os = "linux")]
use xnix::{bind , connect ,rbind ,rconnect };
#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
use win::{bind , connect ,rbind ,rconnect };

fn usage () {
	println!("Cliws - Spawn process IO to websocket with full PTY support");
	println!("https://github.com/b23r0/Cliws");
	println!("Usage: cliws [-p listen port] [-c ws address] [-l reverse port] [-r reverse addr] [command]");
}

fn main() {

	SimpleLogger::new().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

	let arg_count = std::env::args().count();

	if  arg_count == 1{
		usage();
		return;
	}

	let first  = std::env::args().nth(1).expect("parameter not enough");

	match first.as_str() {
		"-l" => {

			let port = std::env::args().nth(2).expect("parameter not enough");
			rbind(port);
			return;
		},
		"-r" => {
			let address = std::env::args().nth(2).expect("parameter not enough");
			let subprocess = std::env::args().nth(3).expect("parameter not enough");
			let mut fullargs : Vec<String> = Vec::new();
			for i in 4..arg_count {
		
				let s = std::env::args().nth(i).expect("parse parameter faild");
				fullargs.push(s);
			}
			rconnect(address, subprocess, fullargs);
			return;
		},
		"-c" => {
			let connect_addr = std::env::args().nth(2).expect("parameter not enough");
			connect(connect_addr);
			return;
		},
		"-p" => {
			let port = std::env::args().nth(2).expect("parameter not enough");
			let mut fullargs : Vec<String> = Vec::new();
			let subprocess = std::env::args().nth(3).expect("parameter not enough");
			for i in 4..arg_count {
		
				let s = std::env::args().nth(i).expect("parse parameter faild");
				fullargs.push(s);
			}
			bind(port, subprocess, fullargs);
			return;
		},

		_ => {
			usage();
			return;
		}
	}
}
