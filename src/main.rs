use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::prelude::FromRawFd;
use std::process::{Command, Stdio};
use nix::libc::{STDIN_FILENO,STDOUT_FILENO};
use nix::pty::openpty;
use nix::sys::{termios};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::{thread};
use websocket::sync::{Server, Writer};
use websocket::{ClientBuilder, OwnedMessage};
use atty::Stream;
use signal_hook::consts::SIGWINCH;
use signal_hook::iterator::Signals;

fn help () {
	println!("Cliws - Run a process and forwarding stdio to websocket");
	println!("https://github.com/b23r0/Cliws");
	println!("Usage: cliws [-p listenport] [-c wsaddress] [command]");
	println!("Example: cliws -p 8000 ping 127.0.0.1");
	println!("         cliws -c ws://127.0.0.1:8000");
}

fn connect( addr : String ){

	let client = ClientBuilder::new(addr.as_str())
	.unwrap()
	.connect_insecure()
	.unwrap();

	let (mut receiver, mut sender) = client.split().unwrap();

	let (tx, rx) = channel();

	let tx_1 = tx.clone();

	let send_loop = thread::spawn(move || {
		loop {
			let message = match rx.recv() {
				Ok(m) => m,
				Err(_) => {
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					let _ = sender.send_message(&message);
					std::process::exit(0);
				},
				OwnedMessage::Binary(_) => {
					let _ = sender.send_message(&message);
				},
				OwnedMessage::Text(_) => {
					let _ = sender.send_message(&message);
				},
				OwnedMessage::Ping(message) => {
					let _ = sender.send_message(&OwnedMessage::Ping(message));
				},
				OwnedMessage::Pong(_) => {},

			}
		}
	});

	let receive_loop = thread::spawn(move || {

		let mut out = unsafe {File::from_raw_fd(1)};

		for message in receiver.incoming_messages() {
			let message = match message {
				Ok(m) => m,
				Err(_) => {
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				}
			};
			match message {
				OwnedMessage::Close(_) => {
					let _ = tx_1.send(OwnedMessage::Close(None));
					return;
				},
				OwnedMessage::Ping(message) => {
					let _ = tx_1.send(OwnedMessage::Pong(message));
				},
				OwnedMessage::Text(message) => {
					out.write_all(message.as_bytes()).unwrap();
				},
				OwnedMessage::Binary(message) => {
					out.write_all(message.as_slice()).unwrap();
				},
				OwnedMessage::Pong(_) => {
					//let _ = tx_1.send(OwnedMessage::Ping([0].to_vec()));
				},
			}
		}
	});

	let mut signals = Signals::new(&[SIGWINCH]).unwrap();

	thread::spawn(move || {

		/*
			from : https://github.com/t57root/amcsh/blob/master/amcsh.c

			void sendws()
			{
				struct winsize ws;
				if( isatty( 0 ) ){
					if( ioctl( 0, TIOCGWINSZ, &ws ) < 0 ){
						perror( "ioctl()" );
						return;
					}
				}
				else{
					ws.ws_row = 25;
					ws.ws_col = 80;
				}

				WINCH winch;
				winch.flag[0] = magickey[0];
				winch.flag[1] = magickey[1];
				winch.flag[2] = 's';
				winch.flag[3] = 's';
				winch.ws_row = ws.ws_row;
				winch.ws_col = ws.ws_col;
				wsend(masterfd, &winch, sizeof(winch));
			}
		*/

		for sig in signals.forever() {

			if sig == SIGWINCH {
			}
		}
	});


	/*

		from : https://github.com/t57root/amcsh/blob/master/amcsh.c

		if( isatty( 1 ) ){
			if( tcgetattr( 1, &tp ) < 0 ){
				perror( "tcgetattr()" );
				return 1;
			}

			memcpy( (void *) &tr, (void *) &tp, sizeof( tr ) );

			tr.c_iflag |= IGNPAR;
			tr.c_iflag &= ~(ISTRIP|INLCR|IGNCR|ICRNL|IXON|IXANY|IXOFF);
			tr.c_lflag &= ~(ISIG|ICANON|ECHO|ECHOE|ECHOK|ECHONL|IEXTEN);
			tr.c_oflag &= ~OPOST;

			tr.c_cc[VMIN]  = 1;
			tr.c_cc[VTIME] = 0;

			if( tcsetattr( 1, TCSADRAIN, &tr ) < 0 ){
				perror( "tcsetattr()" );
				return 1;
			}
		}

	*/

	if atty::is(Stream::Stdin) {

		let mut flags = termios::tcgetattr(STDOUT_FILENO).unwrap();

		flags.input_flags |= termios::InputFlags::IGNPAR;
		flags.input_flags &= !{termios::InputFlags::ISTRIP|termios::InputFlags::INLCR|termios::InputFlags::IGNCR|termios::InputFlags::ICRNL|termios::InputFlags::IXON|termios::InputFlags::IXANY|termios::InputFlags::IXOFF};
		flags.local_flags &= !{termios::LocalFlags::ISIG|termios::LocalFlags::ICANON|termios::LocalFlags::ECHO|termios::LocalFlags::ECHOE|termios::LocalFlags::ECHOK|termios::LocalFlags::ECHONL|termios::LocalFlags::IEXTEN};
		flags.output_flags &= !termios::OutputFlags::OPOST;
		flags.control_chars[nix::libc::VMIN] = 1;
		flags.control_chars[nix::libc::VTIME] = 0;

		termios::tcsetattr(STDIN_FILENO, termios::SetArg::TCSANOW, &flags).unwrap();
	}


	let mut fin = unsafe {File::from_raw_fd(0)};

	loop{
		
		let mut buf : [u8;1] = [0];
		let size = fin.read(buf.as_mut()).unwrap();

		if size == 0 {
			break;
		}

		let msg = OwnedMessage::Binary(buf.to_vec());
		match tx.send(msg) {
			Ok(()) => (),
			Err(_) => {
				break;
			}
		}
	}

	let _ = send_loop.join();
	let _ = receive_loop.join();

	return;
}

fn main() {

	let arg_count = std::env::args().count();

	if  arg_count == 1{
		help();
		return;
	}

	let mut subprocess  = std::env::args().nth(1).expect("parameter not enough");
	let mut port = "8000" . to_string();
	let mut set_port_flag = false;

	if subprocess == "-p" {

		port = std::env::args().nth(2).expect("parameter not enough");
		set_port_flag = true;
	}

	if subprocess == "-c" {

		let connect_addr = std::env::args().nth(2).expect("parameter not enough");
		connect(connect_addr);
		return;
	}

	let mut _start = 2;

	if set_port_flag {
		
		subprocess = std::env::args().nth(3).expect("parameter not enough");
		_start = 4;
	}

	let mut fullargs : Vec<String> = Vec::new();
	for i in _start..arg_count {

		let s = std::env::args().nth(i).expect("parse parameter faild");
		fullargs.push(s);
	}

 	let ends = openpty(None, None).expect("openpty failed");
	let master = ends.master;
	let slave = ends.slave;

	let mut builder = Command::new(subprocess);

	if fullargs.len() !=  0 {
		builder.args(fullargs);
	} 

	builder
	.stdin(unsafe { Stdio::from_raw_fd(slave) })
	.stdout(unsafe { Stdio::from_raw_fd(slave) })
	.stderr(unsafe { Stdio::from_raw_fd(slave) })
	.spawn()
	.expect("!commnad::new");

	let ptyin = unsafe { File::from_raw_fd(master) };
	let ptyout = unsafe { File::from_raw_fd(master) };
	
	let rc_writer = Arc::new(Mutex::new(ptyin));
	let rc_reader = Arc::new(Mutex::new(ptyout));

	let history : Vec<u8> = Vec::new();
	let history_lcks = Arc::new(Mutex::new(history)); 

	// key == source port , value == websocket locker
	let senders : HashMap<u16 , Arc<Mutex<Writer<std::net::TcpStream>>>> = HashMap::new();

	let senders_lcks = Arc::new(Mutex::new(senders));
	let send_lck = senders_lcks.clone();
	let reader_lck = rc_reader.clone();
	
	let history_lock = history_lcks.clone();
	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {
			let mut out = reader_lck.lock().unwrap();

			let result = out.read(buf.as_mut());
			let size = result.unwrap();	

			//child process exit
			if size == 0{
				std::process::exit(0);
			}

			{
				history_lock.lock().unwrap().append(buf[..size].to_vec().as_mut());
			}
			

			//let sendmsg = String::from_utf8(buf[..result.unwrap()].to_vec()).unwrap();
			//print!("{}" ,sendmsg);
			let mut map = send_lck.lock().unwrap();
			for i in map.iter_mut(){
				let msg = OwnedMessage::Binary(buf[..size].to_vec());
				match i.1.lock().unwrap().send_message(&msg){
					Ok(p) => p ,
					Err(e) => {
						println!("{}",e);
					}
				};
			}
			buf.fill(0);
		}

	});

	let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let server = Server::bind(listen_addr).expect("!listen");

	for request in server.filter_map(Result::ok) {

		let writer_lck = rc_writer.clone();
		let send_lck = senders_lcks.clone();

		let history_lock = history_lcks.clone();
		thread::spawn( move || {

			let client = request.accept().unwrap();

			let port = client.peer_addr().unwrap().port();

			let (mut receiver, mut sender) = client.split().unwrap();
			{
				let data = history_lock.lock().unwrap();
				let msg =OwnedMessage::Binary(data.to_vec());
				sender.send_message(&msg).unwrap();
			}
			

			let slck = Arc::new(Mutex::new(sender));
			{
				let mut s = send_lck.lock().unwrap();
				s.insert(port , slck.clone());
			}
			
			for message in receiver.incoming_messages() {
				let message = match message {
					Ok(p) => p,
					Err(_) => {
						send_lck.lock().unwrap().remove(&port);
						return;
					},
				};
				
				match message {
					OwnedMessage::Close(_) => {
						// here need remove sender in map
						//let message = OwnedMessage::Close(None);
						//slck.lock().unwrap().send_message(&message).unwrap();
						send_lck.lock().unwrap().remove(&port);
						return;
					},
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						slck.lock().unwrap().send_message(&message).unwrap();
					},
					OwnedMessage::Text(text) => {
						let mut writer = writer_lck.lock().unwrap();
						writer.write_all(text.as_bytes()).unwrap();
						
					},
					OwnedMessage::Binary(data) => {
						let mut writer = writer_lck.lock().unwrap();
						writer.write_all(data.as_slice()).unwrap();
					},
					_ => {},
				}
			}
		});
	}
}
