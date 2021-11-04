include!("utils.rs");

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::size_of;
use std::os::windows::prelude::FromRawHandle;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::{ptr, thread};
use ntapi::winapi::um::wincontypes::HPCON;
use websocket::sync::{Server, Writer};
use websocket::{ClientBuilder, OwnedMessage};
use winapi::um::consoleapi::{AllocConsole, CreatePseudoConsole, GetConsoleMode, SetConsoleMode};
use winapi::um::fileapi::{CreateFileA, OPEN_EXISTING};
use winapi::um::handleapi::CloseHandle;
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES, SECURITY_ATTRIBUTES};
use winapi::um::processenv::{GetStdHandle, SetStdHandle};
use winapi::um::processthreadsapi::{CreateProcessA, InitializeProcThreadAttributeList, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_LIST, STARTUPINFOA, UpdateProcThreadAttribute};
use winapi::um::winbase::{EXTENDED_STARTUPINFO_PRESENT, INFINITE, STARTUPINFOEXA, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::wincon::{DISABLE_NEWLINE_AUTO_RETURN, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleWindow, SetConsoleWindowInfo};
use winapi::um::wincontypes::{COORD, SMALL_RECT};
use winapi::um::winnt::{CHAR, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};
use winapi::um::namedpipeapi::CreatePipe;
use winapi::um::winnt::{HANDLE , PHANDLE};
use winapi::um::synchapi::WaitForSingleObject;

struct SHandle(*mut c_void);
unsafe impl Send for SHandle {}

fn set_termsize(h : HPCON , row : i16 , col : i16){

	let rect = SMALL_RECT{Left : 0 , Top : 0 , Right : col , Bottom : row};
	unsafe { SetConsoleWindowInfo(h , 1 , &rect) };
}

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

	let mut sa = Box::new(SECURITY_ATTRIBUTES{
		nLength : size_of::<SECURITY_ATTRIBUTES>() as u32, 
		lpSecurityDescriptor: ptr::null_mut(),
		 bInheritHandle: 1 
	});

    let mut input_pipe_read: HANDLE = ptr::null_mut();
    let mut input_pipe_write: HANDLE = ptr::null_mut();

    let ret = unsafe {
        CreatePipe(
            &mut input_pipe_read as PHANDLE,
            &mut input_pipe_write as PHANDLE,
            sa.as_mut() as LPSECURITY_ATTRIBUTES,
            0,
        )
    };

	if ret == 0{
		log::error!("create pipe1 faild");
		return;
	}

	let mut output_pipe_read: HANDLE = ptr::null_mut();
    let mut output_pipe_write: HANDLE = ptr::null_mut();

    let ret = unsafe {

        CreatePipe(
            &mut output_pipe_read as PHANDLE,
            &mut output_pipe_write as PHANDLE,
            sa.as_mut() as LPSECURITY_ATTRIBUTES,
            0,
        )
    };

	if ret == 0{
		log::error!("create pipe2 faild");
		return;
	}

	let h_stdout = unsafe { CreateFileA(
		"CONOUT$\0".as_ptr() as *const CHAR, 
		GENERIC_READ | GENERIC_WRITE, 
		FILE_SHARE_READ | FILE_SHARE_WRITE, 
		ptr::null_mut(), 
		OPEN_EXISTING, 
		FILE_ATTRIBUTE_NORMAL, 
		ptr::null_mut()
	)};

	let h_stdin  = unsafe { CreateFileA(
		"CONIN$\0".as_ptr() as *const CHAR, 
		GENERIC_READ | GENERIC_WRITE, 
		FILE_SHARE_READ | FILE_SHARE_WRITE, 
		ptr::null_mut(), 
		OPEN_EXISTING, 
		FILE_ATTRIBUTE_NORMAL, 
		ptr::null_mut()
	)};

	unsafe {
		SetStdHandle(STD_OUTPUT_HANDLE, h_stdout);
        SetStdHandle(STD_ERROR_HANDLE, h_stdout);
        SetStdHandle(STD_INPUT_HANDLE, h_stdin);
	}

	let mut is_alloc_new_console = false;

	if ( unsafe { GetConsoleWindow() } == ptr::null_mut()){
			unsafe { AllocConsole() };
			//unsafe { ShowWindow(GetConsoleWindow(), SW_HIDE) };
			is_alloc_new_console = true;
	}
	let stdout : HANDLE = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
	let mut console_mode = Box::new(0);
	let ret = unsafe{ GetConsoleMode(
		stdout, 
		console_mode.as_mut() as *mut _ as *mut u32
	)};

	if ret == 0 {
		log::error!("could not get console mode");
		return; 
	}

	*console_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;

	let ret = unsafe { SetConsoleMode(stdout, *console_mode) };

	if ret == 0 {
		log::error!("could not set console mode");
		return;
	}

	let console_coord = COORD{X: 30 , Y:90};
	let mut h_pcon = HPCON::from(ptr::null_mut());

	let ret = unsafe { CreatePseudoConsole(
		console_coord, 
		input_pipe_read, 
		output_pipe_write, 
		0, 
		&mut h_pcon as *mut *mut _ as *mut *mut c_void
	)};

	if ret != 0 {
		log::error!("could not create psuedo console");
		return;
	}

    let mut start_info_ex: STARTUPINFOEXA = unsafe { std::mem::zeroed() };
    start_info_ex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXA>() as u32;

    let mut lp_size: usize = 0;
    let mut ret = unsafe {
        InitializeProcThreadAttributeList(
            ptr::null_mut(),
            1,
            0,
            &mut lp_size,
        )
    };

	if ret != 0 || lp_size == 0 {
		log::error!("could not calculate the number of bytes for the attribute list");
		return;
	}

    let mut lp_attribute_list: Box<[u8]> = vec![0; lp_size].into_boxed_slice();
    start_info_ex.lpAttributeList = lp_attribute_list.as_mut_ptr() as *mut PROC_THREAD_ATTRIBUTE_LIST;

    ret = unsafe {
		InitializeProcThreadAttributeList(
			start_info_ex.lpAttributeList, 
			1, 
			0, 
			&mut lp_size) 
	};
    
	if ret == 0 {
        log::error!("could not setup attribute list");
		return;
    }

    ret = unsafe {
        UpdateProcThreadAttribute(
            start_info_ex.lpAttributeList,
            0,
            0x00020016,
            &mut h_pcon as *mut _ as *mut c_void,
            std::mem::size_of::<HPCON>(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    if ret == 0 {
		log::error!("could not setup process attribute");
		return;
    }

	let mut process_infomation = Box::new(PROCESS_INFORMATION{
		hProcess: ptr::null_mut() , 
		hThread: ptr::null_mut(), 
		dwProcessId: 0, 
		dwThreadId: 0 
	});
	let mut security_attr = Box::new(SECURITY_ATTRIBUTES{
		nLength: 0, 
		lpSecurityDescriptor: ptr::null_mut(), 
		bInheritHandle: 0 
	});

	security_attr.nLength = size_of::<SECURITY_ATTRIBUTES>() as u32;

	let mut security_attr_out = Box::new(SECURITY_ATTRIBUTES{
		nLength: 0, 
		lpSecurityDescriptor: ptr::null_mut(), 
		bInheritHandle: 0 
	});

	{*security_attr_out}.nLength = size_of::<SECURITY_ATTRIBUTES>() as u32;

	let sargs = fullargs.join(" ");
	let command_line = subprocess + " " + sargs.as_str();

	let c_subprocess = CString::new(command_line).unwrap();

	let ret = unsafe { CreateProcessA(
		ptr::null_mut(), 
		c_subprocess.as_c_str().as_ptr() as *mut i8, 
		security_attr.as_mut() as *mut SECURITY_ATTRIBUTES , 
		security_attr_out.as_mut() as *mut SECURITY_ATTRIBUTES , 
		0, 
		EXTENDED_STARTUPINFO_PRESENT, 
		ptr::null_mut(), 
		ptr::null_mut(), 
		&mut start_info_ex as *mut _ as *mut STARTUPINFOA, 
		process_infomation.as_mut() as *mut PROCESS_INFORMATION) 
	};

	let h_child_lck = Arc::new(Mutex::new(SHandle(process_infomation.hProcess)));

	thread::spawn(move || {
		unsafe { WaitForSingleObject(h_child_lck.lock().unwrap().0, INFINITE) };
		log::warn!("child process exit!");
		std::process::exit(0);
	});

	if ret == 0 {
		log::error!("could not create process");
		return;
	}

	if input_pipe_read != ptr::null_mut() {
		unsafe { CloseHandle(input_pipe_read) };
	}

	if output_pipe_write != ptr::null_mut() {
		unsafe { CloseHandle(output_pipe_write) };
	}

	let ptyin = unsafe { File::from_raw_handle(input_pipe_write) };
	let mut ptyout = unsafe { File::from_raw_handle(output_pipe_read) };
	
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

	let hcon_lcks = Arc::new(Mutex::new(SHandle(h_pcon)));

	for request in server.filter_map(Result::ok) {

		let writer_lck = rc_writer.clone();
		let send_lck = senders_lcks.clone();

		let history_lock = history_lcks.clone();
		let hcon_lck = hcon_lcks.clone();
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

						if data[0] == MAGIC_FLAG[0] && data[1] == MAGIC_FLAG[1] {
							let h = hcon_lck.lock().unwrap();
							set_termsize(h.0 , data[2] as i16, data[3] as i16 );
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