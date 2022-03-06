// use libc::{self};
use nix::pty::openpty;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::process::{Child, Command, Stdio};
use std::fs::File;
// use std::prelude::*;

#[derive(Debug)]
pub struct Pty {
    process: Child,
    fd: i32,
}

fn create_pty(process: &str) -> Pty {
    let ends = openpty(None, None).expect("openpty failed");
    let master = ends.master;
    let slave = ends.slave;

    let mut builder = Command::new(process);
    builder.stdin(unsafe { Stdio::from_raw_fd(slave) });
    builder.stdout(unsafe { Stdio::from_raw_fd(slave) });
    builder.stderr(unsafe { Stdio::from_raw_fd(slave) });

    match builder.spawn() {
        Ok(process) => {
            let pty = Pty {
                process,
                fd: master,
            };

            pty
        }
        Err(e) => {
            panic!("Failed to create pty: {}", e);
        }
    }
}

fn main() {
    let shell = "/bin/bash";

    let pty = create_pty(shell);
    println!("{:?}", pty);

    let mut output = unsafe { File::from_raw_fd(pty.fd) };
    let test_str = "touch /tmp/itworks\n".as_bytes();
    output.write(test_str);
    // write!(output, "touch /tmp/itworks\n");
    output.sync_all(); // was: flush

    // std::thread::sleep_ms(1000);

    println!("{}", pty.process.id());
}
