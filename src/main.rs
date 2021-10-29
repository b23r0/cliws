use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::process::{Child, Command, Stdio, exit};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::{thread};
use sl_console::{con_init, conin, conout};
use websocket::sync::{Server, Writer};
use websocket::{ClientBuilder, OwnedMessage};

fn help () {
	println!("Cliws - Run a process and forwarding stdio to websocket");
	println!("https://github.com/b23r0/Cliws");
	println!("Usage: cliws [-p listenport] [-c wsaddress] [command]");
	println!("Example: cliws -p 8000 ping 127.0.0.1");
	println!("         cliws -c ws://127.0.0.1:8000");
}

fn read_line() -> io::Result<Option<String>> {
    let mut buf = Vec::with_capacity(30);

    for c in conin().bytes() {
        match c {
            Err(e) => return Err(e),
            Ok(0) | Ok(3) | Ok(4) => return Ok(None),
            Ok(0x7f) => {
                buf.pop();
            }
            Ok(b'\n') | Ok(b'\r') =>{ 
				buf.push(b'\n');
				break;
			},
            Ok(c) => buf.push(c),
        }
    }

    let string =
        String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(string))
}

fn connect( addr : String ){


	con_init().unwrap();

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
		let mut conout = conout();
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
					conout.write_all(message.as_bytes()).unwrap();
				},
				OwnedMessage::Binary(message) => {
					conout.write_all(message.as_slice()).unwrap();
				},
    			OwnedMessage::Pong(_) => {
					//let _ = tx_1.send(OwnedMessage::Ping([0].to_vec()));
				},
			}
		}
	});
	

	loop {
		let text = read_line().unwrap();
		let msg = OwnedMessage::Text(text.unwrap());
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

	let mut cmd : Child;

	if fullargs.len() ==  0 {
		cmd = Command::new(subprocess)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("!commnad::new");
	} else {
		cmd = Command::new(subprocess).args(fullargs)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("!commnad::new");
	}



	let out = Arc::new(Mutex::new(cmd.stdout.take().expect("!stdout")));
	let err = Arc::new(Mutex::new(cmd.stderr.take().expect("!stdout")));
	let writer = Arc::new(Mutex::new(cmd.stdin.take().expect("!stdin")));

	let stdout_lck = out.clone();
	let stderr_lck = err.clone();

	// key == source port , value == websocket locker
	let senders : HashMap<u16 , Arc<Mutex<Writer<std::net::TcpStream>>>> = HashMap::new();

	let senders_lcks = Arc::new(Mutex::new(senders));
	let send_lck = senders_lcks.clone();
	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {
			let mut out = stdout_lck.lock().unwrap();
			let result = out.read(buf.as_mut());

			let size = result.unwrap();

			//child process exit
			if size == 0{
				std::process::exit(0);
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

	let send_lck = senders_lcks.clone();
	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {
			let mut err = stderr_lck.lock().unwrap();
			let result = err.read(buf.as_mut());
			let size = result.unwrap();

			//child process exit
			if size == 0{
				exit(0);
			}
			//let sendmsg = String::from_utf8(buf[..size].to_vec()).unwrap();
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

		let writer_lck = writer.clone();
		let send_lck = senders_lcks.clone();
		thread::spawn( move || {

			let client = request.accept().unwrap();

			let port = client.peer_addr().unwrap().port();

			let (mut receiver, sender) = client.split().unwrap();
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
