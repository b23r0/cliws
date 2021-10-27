use std::process::{Child, Command , ChildStdout, Stdio };
use std::io::{Error, Read, Write};

pub struct Pio {
    command: String,
    args : String,
    child : Option<Child>
}

impl Pio {
    pub fn new() -> Pio {
        Pio {
            command: String::from(""),
            args : String::from(""),
            child : None
        }
    }

    pub fn set(& mut self, process_name : String , args : String) {
        self.command = process_name;
        self.args = args;
    }

    pub fn run(&mut self) {
        let cmd = &mut Command::new(self.command.as_str());

        if self.command == "" {
            panic!("command is empty")
        }

        if self.args == ""{
            self.child = Some(cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute child"));
            
        }
        else {
            self.child = Some(cmd.stdin(Stdio::piped()).arg(self.args.as_str())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute child"));
        }
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