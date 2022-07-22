use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
use windows::Win32::System::Console::{SetConsoleMode, GetConsoleMode, GetConsoleScreenBufferInfo, CONSOLE_SCREEN_BUFFER_INFO};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::prelude::{FromRawHandle};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use websocket::sync::{Server, Writer};
use websocket::{ClientBuilder, OwnedMessage};

use crate::conpty::{console, self};

use crate::utils::{MAGIC_FLAG, makeword, splitword, get_stdout_handle, get_stdin_handle, handle_to_rawhandle};

pub fn rconnect( addr : String , subprocess : String , fullargs : Vec<String>){

	let client = match  { 
		match ClientBuilder::new(addr.as_str()){
			Err(_) => {
				log::error!("parse address [{}] faild. eg : ws://127.0.0.1:8000" , addr);
				return;
			},
			Ok(p) => p
		}
	}.connect_insecure() {
		Err(_) => {
			log::error!("connect [{}] faild" , addr);
			return;
		},
		Ok(p) => p
	};

	let (mut receiver, mut sender) = match client.split() {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	let (tx, rx) = channel();

	let tx_1 = tx.clone();

	let full_cmd = format!("{} {}" ,subprocess , fullargs.join(" ").as_str());

	log::info!("start process: [{}]" ,full_cmd );

	let proc = match conpty::spawn(full_cmd) {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	let pid = proc.pid();

	let mut ptyin = match proc.input() {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};
	let mut ptyout = match proc.output()  {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	thread::spawn(move || {
		let handle = unsafe { OpenProcess(windows::Win32::System::Threading::PROCESS_ALL_ACCESS, false, pid) };

		if handle.is_invalid() {
			log::error!("OpenProcess error");
			std::process::exit(0);
		} else {
			unsafe { WaitForSingleObject(handle, 0xffffffff)};
		}
		log::warn!("child process exit!");
		std::process::exit(0);

	});

	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {

			let result = ptyout.read(buf.as_mut());
			let size = match result {
				Err(e) => {
					log::error!("error : {}" , e);
					std::process::exit(0);
				},
				Ok(p) => p
			};

			
			if size == 0 {
				break;
			}

			let msg = OwnedMessage::Binary(buf[..size].to_vec());
			match tx.send(msg) {
				Ok(()) => (),
				Err(_) => {
					break;
				}
			}
			buf.fill(0);
		}

	});

	thread::spawn(move || {
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
			OwnedMessage::Text(text) => {
				match ptyin.write_all(text.as_bytes()) {
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
				
			},
			OwnedMessage::Binary(data) => {

				if data.len() == 6 && data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {

    						let row = makeword(data[2] , data[3]);
    						let col = makeword(data[4] , data[5]);

    						proc.resize(col as i16 , row as i16).unwrap();
    						continue;
    					}

				match ptyin.write_all(data.as_slice()) {
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
			},
			OwnedMessage::Pong(_) => {
				//let _ = tx_1.send(OwnedMessage::Ping([0].to_vec()));
			},
		}
	}

	
}

pub fn rbind(port : String){

	log::info!("listen to: [{}:{}]" ,"0.0.0.0" , port );
	let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let mut server = match Server::bind(listen_addr) {
		Err(_) => {
			log::error!("bind [0.0.0.0:{}] faild" , port);
			return;
		}, 
		Ok(p) => p
	};

	let request = match server.accept(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e.error);
			return;
		},
	};
	let client = match request.accept(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e.1);
			return;
		},
	};

	let addr = match client.peer_addr(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
	};

	let ip = addr.ip();
	let port = addr.port();

	log::info!("accept from : [{}:{}]" ,ip , port );

	let (mut receiver, sender) = match client.split(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
	};
	

	let slck_1 = Arc::new(Mutex::new(sender));
	let slck_2 = slck_1.clone();
	let slck_3 = slck_1.clone();
	
	let mut mode = windows::Win32::System::Console::CONSOLE_MODE::default();
	let h = get_stdout_handle();
	let ret = unsafe { GetConsoleMode(h, &mut mode)};

	if ret == false {
		log::error!("get console mode faild!");
		std::process::exit(0);
	}
	

	let console = match console::Console::current() {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};
	match console.set_raw() {
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	thread::spawn(move || {

		let h = std::os::windows::prelude::RawHandle::from(handle_to_rawhandle(&get_stdin_handle()));
		let mut fin = unsafe {File::from_raw_handle(h)};

		loop{
			
			let mut buf : [u8;1] = [0];
			let size = match fin.read(buf.as_mut()){
				Ok(p) => p,
				Err(e) => {
					log::error!("error : {}" , e);
					std::process::exit(0);
				},
			};

			if size == 0 {
				break;
			}

			let msg = OwnedMessage::Binary(buf.to_vec());
			match slck_2.lock().unwrap().send_message(&msg) {
				Err(e) => {
					log::error!("error : {}" , e);
					std::process::exit(0);
				},
				Ok(p) => p
			};
		}
	});

	thread::spawn( move ||{
		let mut row = 0 ;
		let mut col = 0;

		let h = get_stdout_handle();
		
		loop {
			let mut csbi: windows::Win32::System::Console::CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
			let ret = unsafe { GetConsoleScreenBufferInfo( h  , &mut csbi)};

			if ret == false {
				log::error!("get console size faild!");
				unsafe {SetConsoleMode(h , mode)};
				std::process::exit(0);
			}

			if row != csbi.srWindow.Bottom || col != csbi.srWindow.Right {

				let (bottom1 , bottom2) = splitword((csbi.srWindow.Bottom + 1) as u16);
				let (right1 , right2) = splitword((csbi.srWindow.Right + 1) as u16);
				
				let vec = [MAGIC_FLAG[0], MAGIC_FLAG[1] , bottom1 , bottom2 , right1 , right2 ];
				
				let msg = OwnedMessage::Binary(vec.to_vec());
				match slck_3.lock().unwrap().send_message(&msg) {
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};

				row = csbi.srWindow.Bottom;
				col = csbi.srWindow.Right;
			} 

			thread::sleep(time::Duration::from_secs(1));
		}
		
	} );

	let h = std::os::windows::prelude::RawHandle::from(handle_to_rawhandle(&get_stdout_handle()));

	let mut out = unsafe {File::from_raw_handle(h)};

	for message in receiver.incoming_messages() {
		let message = match message {
			Ok(p) => p,
			Err(_) => {
				log::warn!("client closed : [{}:{}]" ,ip , port );

				let h = get_stdout_handle();

				unsafe {SetConsoleMode(h , mode)};
				std::process::exit(0);
			},
		};
		
		match message {
			OwnedMessage::Close(_) => {
				log::warn!("client closed : [{}:{}]" ,ip , port );
				unsafe {SetConsoleMode(get_stdout_handle() , mode)};
				std::process::exit(0);
			},
			OwnedMessage::Ping(ping) => {
				let message = OwnedMessage::Pong(ping);
				match slck_1.lock().unwrap().send_message(&message){
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
			},
			OwnedMessage::Text(text) => {
				match out.write_all(text.as_bytes()){
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
				
			},
			OwnedMessage::Binary(data) => {
				match out.write_all(data.as_slice()){
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
			},
			_ => {},
		}
	}
}
pub fn connect( addr : String ){

	let client = match  { 
		match ClientBuilder::new(addr.as_str()){
			Err(_) => {
				log::error!("parse address [{}] faild. eg : ws://127.0.0.1:8000" , addr);
				return;
			},
			Ok(p) => p
		}
	}.connect_insecure() {
		Err(_) => {
			log::error!("connect [{}] faild" , addr);
			return;
		},
		Ok(p) => p
	};

	let mut mode = windows::Win32::System::Console::CONSOLE_MODE::default();
	
	let ret = unsafe { GetConsoleMode(get_stdout_handle(), &mut mode)};

	if ret == false {
		log::error!("get console mode faild!");
		std::process::exit(0);
	}
	

	let console = match console::Console::current(){
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	match console.set_raw(){
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	let (mut receiver, mut sender) = match client.split(){
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

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
					unsafe {SetConsoleMode(get_stdout_handle() , mode)};
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

	let tx_2 = tx.clone();
	thread::spawn( move ||{
		let mut row = 0 ;
		let mut col = 0;

		let h = get_stdout_handle();
		loop {
			let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
			let ret = unsafe { GetConsoleScreenBufferInfo( h  , &mut csbi)};

			if ret == false {
				log::error!("get console size faild!");
				unsafe {SetConsoleMode(h , mode)};
				std::process::exit(0);
			}

			if row != csbi.srWindow.Bottom || col != csbi.srWindow.Right {

				let (bottom1 , bottom2) = splitword((csbi.srWindow.Bottom + 1) as u16);
				let (right1 , right2) = splitword((csbi.srWindow.Right + 1) as u16);
				
				let vec = [MAGIC_FLAG[0], MAGIC_FLAG[1] , bottom1 , bottom2 , right1 , right2 ];
				
				let msg = OwnedMessage::Binary(vec.to_vec());
				match tx_2.send(msg) {
					Ok(()) => (),
					Err(_) => {
						break;
					}
				}

				row = csbi.srWindow.Bottom;
				col = csbi.srWindow.Right;
			} 

			thread::sleep(time::Duration::from_secs(1));
		}
		
	} );

	let receive_loop = thread::spawn(move || {

		let h = get_stdout_handle();

		let mut out = unsafe {File::from_raw_handle(handle_to_rawhandle(&h))};

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
					match out.write_all(message.as_bytes()){
						Err(e) => {
							log::error!("error : {}" , e);
							std::process::exit(0);
						},
						Ok(p) => p
					};
				},
				OwnedMessage::Binary(message) => {
					match out.write_all(message.as_slice()){
						Err(e) => {
							log::error!("error : {}" , e);
							std::process::exit(0);
						},
						Ok(p) => p
					};
				},
				OwnedMessage::Pong(_) => {
					//let _ = tx_1.send(OwnedMessage::Ping([0].to_vec()));
				},
			}
		}
	});
	
	// first set terminal size
	let mut input = unsafe {File::from_raw_handle(handle_to_rawhandle(&get_stdin_handle()))};

	loop{
		
		let mut buf : [u8;1] = [0];
		let size = match input.read(buf.as_mut()){
			Err(e) => {
				log::error!("error : {}" , e);
				return;
			},
			Ok(p) => p
		};

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

	
}

pub fn bind(port : String , subprocess : String , fullargs : Vec<String>) {

	log::info!("listen to: [{}:{}]" ,"0.0.0.0" , port );
	let listen_addr = format!("{}:{}", "0.0.0.0", port);

	let mut server = match Server::bind(listen_addr) {
		Err(_) => {
			log::error!("bind [0.0.0.0:{}] faild" , port);
			return;
		}, 
		Ok(p) => p
	};

	let request = match server.accept(){
		Err(e) => {
			log::error!("error : {}" , e.error);
			return;
		},
		Ok(p) => p
	};
	let client = match request.accept(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e.1);
			return;
		},
	};

	let addr = match client.peer_addr(){
		Ok(p) => p,
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
	};

	let ip = addr.ip();
	let port = addr.port();

	log::info!("accept from : [{}:{}]" ,ip , port );

	let full_cmd = format!("{} {}" ,subprocess , fullargs.join(" ").as_str());

	let proc = conpty::spawn(full_cmd).unwrap();
	let pid = proc.pid();

	let mut ptyin = match proc.input(){
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};
	let mut ptyout = match proc.output(){
		Err(e) => {
			log::error!("error : {}" , e);
			return;
		},
		Ok(p) => p
	};

	thread::spawn(move || {
		let handle = unsafe { OpenProcess(windows::Win32::System::Threading::PROCESS_ALL_ACCESS, false, pid) };

		if handle.is_invalid(){
			log::error!("OpenProcess error");
			std::process::exit(0);
		}

		unsafe { WaitForSingleObject(handle, 0xffffffff)};

		log::warn!("child process exit!");
		std::process::exit(0);

	});

	let history : Vec<u8> = Vec::new();
	let history_lck1 = Arc::new(Mutex::new(history)); 

	let senders : HashMap<u16 , Arc<Mutex<Writer<std::net::TcpStream>>>> = HashMap::new();

	let sender_lck1 = Arc::new(Mutex::new(senders));
	let sender_lck2 = sender_lck1.clone();
	
	let history_lck2 = history_lck1.clone();

	thread::spawn(move || {

		let mut buf : [u8;1024] = [0;1024];
		loop {

			let result = ptyout.read(buf.as_mut());
			let size = match result {
				Ok(p) => {
					if p == 0{
						std::process::exit(0);
					}
					p
				},
				Err(e) => {
					log::error!("error : {}" , e);
					std::process::exit(0);
				},
			};

			{ history_lck2.lock().unwrap().append(buf[..size].to_vec().as_mut()); }
			
			let mut map = sender_lck2.lock().unwrap();
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

	let (mut receiver, mut sender) = client.split().unwrap();
	{
		let data = history_lck1.lock().unwrap();
		let msg =OwnedMessage::Binary(data.to_vec());
		sender.send_message(&msg).unwrap();
	}
	

	let slck = Arc::new(Mutex::new(sender));
	{
		let mut s = sender_lck1.lock().unwrap();
		s.insert(port , slck.clone());
	}
	
	for message in receiver.incoming_messages() {
		let message = match message {
			Ok(p) => p,
			Err(_) => {
				log::warn!("client closed : [{}:{}]" ,ip , port );
				sender_lck1.lock().unwrap().remove(&port);
				return;
			},
		};
		
		match message {
			OwnedMessage::Close(_) => {
				sender_lck1.lock().unwrap().remove(&port);
				return;
			},
			OwnedMessage::Ping(ping) => {
				let message = OwnedMessage::Pong(ping);
				slck.lock().unwrap().send_message(&message).unwrap();
			},
			OwnedMessage::Text(text) => {
				ptyin.write_all(text.as_bytes()).unwrap();
			},
			OwnedMessage::Binary(data) => {

				if data.len() == 6 && data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {

    						let row = makeword(data[2] , data[3]);
    						let col = makeword(data[4] , data[5]);

    						match proc.resize(col as i16 , row as i16){
    							Err(e) => {
    								log::error!("error : {}" , e);
    								std::process::exit(0);
    							},
    							Ok(p) => p
    						};
    						continue;
    					}
				match ptyin.write_all(data.as_slice()){
					Err(e) => {
						log::error!("error : {}" , e);
						std::process::exit(0);
					},
					Ok(p) => p
				};
			},
			_ => {},
		}
	}
}