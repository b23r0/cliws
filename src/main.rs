use std::process::ChildStdout;
use std::sync::{Arc, Mutex};
use std::thread;
use websocket::sync::Server;
use websocket::OwnedMessage;

mod pio;
use pio::Pio;

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

    let mut subprocess  = std::env::args().nth(1)
								.expect("parameter not enough");

    let mut port = "8000" . to_string();

	let mut set_port_flag = false;

    if subprocess == "-p" {

        port = std::env::args().nth(2)
						.expect("parameter not enough");
		set_port_flag = true;
    }

	let mut _start = 2;

	if set_port_flag {
		
		subprocess = std::env::args().nth(3)
							.expect("parameter not enough");

		_start = 4;
	}

	let mut fullargs = String::from("");
	for i in _start..arg_count {

		let s = std::env::args().nth(i)
							.expect("parse parameter faild");

		fullargs += &s;
		fullargs += &String::from(" ");
	}

	let mut databuf : Vec<Vec<u8>> = Vec::new();

    let mut pio = Pio::new();

    pio.set(subprocess, fullargs);
    pio.run();

	let databuf_lock_sub = Arc::new(Mutex::new(databuf));
	let pio_lock_sub = Arc::new(Mutex::new(pio));
	let pio_lock = Arc::clone(&pio_lock_sub);
	let databuf_lock = databuf_lock_sub.clone();

	thread::spawn(move || {
		let mut buf : [u8;1024] = [0;1024];
		loop {
			let mut _pio = pio_lock.lock().unwrap();
			let result = _pio.read(buf.as_mut());
			buf.fill(0);
		}
	});

    let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let server = Server::bind(listen_addr).expect("listen websocket faild");

	for request in server.filter_map(Result::ok) {

		let pio_lock = Arc::clone(&pio_lock_sub);

		thread::spawn( move || {

			let mut client = request.accept().unwrap();

			let (mut receiver, mut sender) = client.split().unwrap();

			for message in receiver.incoming_messages() {
				let message = message.unwrap();

				match message {
					OwnedMessage::Close(_) => {
						let message = OwnedMessage::Close(None);
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						sender.send_message(&message).unwrap();
					}
					OwnedMessage::Text(text) => {
						let mut _pio = pio_lock.lock().unwrap();
						_pio.write(text.as_bytes());
					}
					_ => sender.send_message(&message).unwrap(),
				}
			}
		});
	}
}
