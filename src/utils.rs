use atty::Stream;
use nix::libc;

pub fn get_termsize(fd : i32) -> Option<Box<libc::winsize>> {
	let mut ret = 0;
	let mut size = Box::new(libc::winsize{
		ws_row : 25 , 
		ws_col : 80 ,
		ws_xpixel : 0,
		ws_ypixel: 0, 
		
	});

	if atty::is(Stream::Stdin){
		ret = unsafe {libc::ioctl(fd , libc::TIOCGWINSZ , &mut *size) } as i32;
	} else {
		size.ws_row = 25;
		size.ws_col = 80;
	};

	if ret < 0 {
		return None;
	}

	Some(size)
}

pub fn set_termsize(fd : i32 , mut size : Box<libc::winsize>) -> bool {
	(unsafe {libc::ioctl(fd , libc::TIOCSWINSZ , &mut *size) } as i32) > 0
}