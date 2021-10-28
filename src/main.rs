use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use websocket::sync::{Server, Writer};
use websocket::OwnedMessage;

fn help () {
	println!("Cliws - Run a process and forwarding stdio to websocket");
	println!("https://github.com/b23r0/Cliws");
	println!("Usage: curl [-p listenport] <command>");
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

	let mut _start = 2;

	if set_port_flag {
		
		subprocess = std::env::args().nth(3).expect("parameter not enough");
		_start = 4;
	}

	let mut fullargs = String::from("");
	for i in _start..arg_count {

		let s = std::env::args().nth(i).expect("parse parameter faild");
		fullargs += &s;
		fullargs += &String::from(" ");
	}

	let mut cmd = Command::new(subprocess)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("!commnad::new");

	let out = Arc::new(Mutex::new(cmd.stdout.take().expect("!stdout")));
	let writer = Arc::new(Mutex::new(cmd.stdin.take().expect("!stdin")));

	let stdout_lck = out.clone();

	let senders : Vec<Arc<Mutex<Writer<std::net::TcpStream>>>> = Vec::new();

	let senders_lcks = Arc::new(Mutex::new(senders));
	let send_lck = senders_lcks.clone();
	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {
			let mut out = stdout_lck.lock().unwrap();
			let result = out.read(buf.as_mut());
			let sendmsg = String::from_utf8(buf[..result.unwrap()].to_vec()).unwrap();
			print!("{}" ,sendmsg);

			for i in send_lck.lock().unwrap().iter_mut(){
				let msg = OwnedMessage::Text(sendmsg.clone());
				i.lock().unwrap().send_message(&msg).unwrap();
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

			let (mut receiver, sender) = client.split().unwrap();
			let slck = Arc::new(Mutex::new(sender));
			{
				let mut s = send_lck.lock().unwrap();
				s.push(slck.clone());
			}
			
			for message in receiver.incoming_messages() {
				let message = message.unwrap();
				
				match message {
					OwnedMessage::Close(_) => {
						// here need remove sender in vec
						let message = OwnedMessage::Close(None);
						slck.lock().unwrap().send_message(&message).unwrap();
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						slck.lock().unwrap().send_message(&message).unwrap();
					}
					OwnedMessage::Text(text) => {
						let mut writer = writer_lck.lock().unwrap();
						writer.write_all(text.as_bytes()).unwrap();
						writer.write_all("\n".as_bytes()).unwrap();
					}
					_ => slck.lock().unwrap().send_message(&message).unwrap(),
				}
			}
		});
	}
}
