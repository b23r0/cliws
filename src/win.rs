include!("utils.rs");

use conpty::{Process};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::prelude::FromRawHandle;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::{ptr, thread};
use websocket::sync::{Server, Writer};
use websocket::{ClientBuilder, OwnedMessage};
use winapi::um::processthreadsapi::{OpenProcess};
use winapi::um::consoleapi::{AllocConsole};
use winapi::um::processenv::{GetStdHandle};
use winapi::um::winbase::{INFINITE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::winnt::{PROCESS_ALL_ACCESS};
use winapi::um::synchapi::WaitForSingleObject;

struct ProcWrapper(Process);
unsafe impl Send for ProcWrapper {}

pub fn rconnect( addr : String , subprocess : String , fullargs : Vec<String>){}
pub fn rbind(port : String){}
pub fn connect( addr : String ){

	unsafe { AllocConsole() };

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

		let h = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

		let mut out = unsafe {File::from_raw_handle(h)};

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
	
	// first set terminal size
	let h = unsafe { GetStdHandle(STD_INPUT_HANDLE) };

	let mut input = unsafe {File::from_raw_handle(h)};

	loop{
		
		let mut buf : [u8;1] = [0];
		let size = input.read(buf.as_mut()).unwrap();

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

pub fn bind(port : String , subprocess : String , fullargs : Vec<String>) {

	let full_cmd = subprocess + fullargs.join(" ").as_str();

	let proc = conpty::spawn(full_cmd).unwrap();
	let pid = proc.pid();

	let ptyin = proc.input().unwrap();
	let mut ptyout = proc.output().unwrap();

	let proc_lck = Arc::new(Mutex::new(ProcWrapper(proc)));

	thread::spawn(move || {
		let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };

		if handle != ptr::null_mut() {
			unsafe { WaitForSingleObject(handle, INFINITE)};
		}
		log::warn!("child process exit!");
		std::process::exit(0);

	});
	
	let rc_writer = Arc::new(Mutex::new(ptyin));

	let history : Vec<u8> = Vec::new();
	let history_lcks = Arc::new(Mutex::new(history)); 

	let senders : HashMap<u16 , Arc<Mutex<Writer<std::net::TcpStream>>>> = HashMap::new();

	let senders_lcks = Arc::new(Mutex::new(senders));
	let send_lck = senders_lcks.clone();
	
	let history_lock = history_lcks.clone();
	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {

			let result = ptyout.read(buf.as_mut());
			let size = result.unwrap();	

			if size == 0{
				std::process::exit(0);
			}

			{ history_lock.lock().unwrap().append(buf[..size].to_vec().as_mut()); }
			
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

	log::info!("listen to: [{}:{}]" ,"0.0.0.0" , port );
	let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let server = Server::bind(listen_addr).expect("!listen");

	for request in server.filter_map(Result::ok) {

		let writer_lck = rc_writer.clone();
		let send_lck = senders_lcks.clone();

		let history_lock = history_lcks.clone();
		let proc_lck1 = proc_lck.clone();
		thread::spawn( move || {

			let client = request.accept().unwrap();

			let port = client.peer_addr().unwrap().port();
			let ip = client.peer_addr().unwrap().ip();

			log::info!("accept from : [{}:{}]" ,ip , port );

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
						log::warn!("client closed : [{}:{}]" ,ip , port );
						send_lck.lock().unwrap().remove(&port);
						return;
					},
				};
				
				match message {
					OwnedMessage::Close(_) => {
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

						if data.len() == 4{

							if data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {
	
								let row = data[2] as i16;
								let col = data[3] as i16;

								proc_lck1.lock().unwrap().0.resize(col , row).unwrap();
								continue;
							}
						}
						
						let mut writer = writer_lck.lock().unwrap();
						writer.write_all(data.as_slice()).unwrap();
					},
					_ => {},
				}
			}
		});
	}
}