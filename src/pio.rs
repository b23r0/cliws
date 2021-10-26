use std::process::{Child, Command, Stdio};
use std::io::{Read, Write ,Error};

pub struct Pio {
    command: String,
    args : String,
    child : Option<Child>
}

impl Pio {
    pub fn new(process_name : String , args : String) -> Pio {
        Pio {
            command: process_name,
            args : args,
            child : None
        }
    }

    pub fn run(&mut self) {
        let mut child = Command::new(self.command.as_str()).arg(self.args.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute child");
        self.child = Some(child);
    }

    pub fn write(&mut self , buf: &[u8]) {
        let a = self.child.as_ref().expect("get subprocess fd faild");
        let b = a.stdin.as_ref();
        b.unwrap().write_all(buf);
    }
    pub fn read(&mut self ,  buf:& mut[u8]) -> Result<usize , Error> {
        let a = self.child.as_mut().expect("get subprocess fd faild");
        let mut b = a.stdout.as_mut();
        b.unwrap().read(buf)
    }
}