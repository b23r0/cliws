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
	
	return;

    let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let server = Server::bind(listen_addr).expect("listen websocket faild");

	for request in server.filter_map(Result::ok) {
		// Spawn a new thread for each connection.
		thread::spawn(|| {
			if !request.protocols().contains(&"rust-websocket".to_string()) {
				request.reject().unwrap();
				return;
			}

			let mut client = request.use_protocol("rust-websocket").accept().unwrap();

			let ip = client.peer_addr().unwrap();

			println!("Connection from {}", ip);

			let message = OwnedMessage::Text("Hello".to_string());
			client.send_message(&message).unwrap();

			let (mut receiver, mut sender) = client.split().unwrap();

			for message in receiver.incoming_messages() {
				let message = message.unwrap();

				match message {
					OwnedMessage::Close(_) => {
						let message = OwnedMessage::Close(None);
						sender.send_message(&message).unwrap();
						println!("Client {} disconnected", ip);
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						sender.send_message(&message).unwrap();
					}
					_ => sender.send_message(&message).unwrap(),
				}
			}
		});
	}
}
