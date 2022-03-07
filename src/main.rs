use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::{
    fs::File,
    io::Read,
    io::Write,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    thread,
};

use futures::{
    channel::mpsc::{self, Receiver},
    executor, future,
    stream::StreamExt,
};

use nix::{
    pty::{openpty, Winsize},
    unistd::setsid,
};

use termion::{
    self,
    event::Event,
    input::{EventsAndRaw, TermReadEventsAndRaw},
    raw::IntoRawMode,
};

mod ioctl {
    nix::ioctl_none_bad!(set_controlling, libc::TIOCSCTTY);
    nix::ioctl_write_ptr_bad!(win_resize, libc::TIOCSWINSZ, nix::pty::Winsize);
}

/// An update from a PTY.
#[derive(Debug, PartialEq, Eq)]
pub enum PtyUpdate {
    /// The PTY has closed the file.
    Exited,
    /// PTY sends byte.
    Byte(u8),
}

pub fn get_term_size() -> std::io::Result<Winsize> {
    let (cols, rows) = termion::terminal_size()?;
    Ok(Winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    })
}

fn input_to_stream(mut input_events: EventsAndRaw<File>) -> Receiver<(Event, Vec<u8>)> {
    let (mut send, recv) = mpsc::channel(0x1000);
    thread::spawn(move || {
        while let Some(Ok((e, d))) = input_events.next() {
            send.try_send((e, d)).unwrap();
        }
        send.disconnect();
    });
    recv
}

async fn event_loop<W: Write>(
    input_stream: &mut Receiver<(Event, Vec<u8>)>, // Events from the keyboard coming into the termspy process
    tty_output: &mut W,                            // Output to the terminal
    pty_master_file: &mut File,
) {
    let (mut send, mut pty_update_recv) = futures::channel::mpsc::channel(0x1000);

    let thread_file = pty_master_file.try_clone().unwrap();

    thread::spawn(move || {
        let mut pty_output = thread_file.bytes();
        while let Some(Ok(byte)) = pty_output.next() {
            executor::block_on(future::poll_fn(|cx| send.poll_ready(cx))).unwrap();
            send.start_send(PtyUpdate::Byte(byte)).unwrap();
        }
        send.try_send(PtyUpdate::Exited).unwrap();
        send.disconnect();
    });

    loop {
        futures::select! {
            input = input_stream.next() => {
                    match input {
                        Some((_event, data)) => {
                            // HOOK HERE: To get input from user coming into the terminal
                            match pty_master_file.write_all(data.as_slice()) {
                                Ok(_) => (),
                                Err(e) => {
                                    panic!("Error writing to pty: {}", e);
                                }
                            }
                            match pty_master_file.flush() {
                                Ok(_) => (),
                                Err(e) => {
                                    panic!("Error flushing pty: {}", e);
                                }
                            }
                        },
                        None => unreachable!(),
                    }

            },
            output = pty_update_recv.next() => {
                match output {
                    Some(PtyUpdate::Exited) => {
                        break;
                    },
                    Some(PtyUpdate::Byte(byte)) => {
                        // HOOK HERE: To get the output from the PTY
                        match tty_output.write_all(&[byte]) {
                            Ok(_) => (),
                            Err(e) => {
                                panic!("Error writing to tty: {}", e);
                            }
                        }
                        match tty_output.flush() {
                            Ok(_) => (),
                            Err(e) => {
                                panic!("Error flushing tty: {}", e);
                            }
                        }
                    },
                    None => unreachable!(),
                }
            },
        }
    }
}

fn main() -> std::io::Result<()> {
    let is_tty = termion::is_tty(&std::io::stdin().as_raw_fd());
    if !is_tty {
        panic!("Not a tty");
    }

    let mut tty_output = termion::get_tty()?.into_raw_mode()?;
    let input_events = tty_output.try_clone()?.events_and_raw();
    let mut input_stream = input_to_stream(input_events);

    let size = get_term_size()?;
    let pty = openpty(&size, None).unwrap();
    #[allow(unused_assignments)]
    let mut pty_master_file: Option<File> = None;

    unsafe {
        pty_master_file = Some(File::from_raw_fd(pty.master));

        match Command::new("/bin/zsh")
            .args(["-i"])
            .stdin(Stdio::from_raw_fd(pty.slave))
            // .stdout(stdout_pipe)
            .stdout(Stdio::from_raw_fd(pty.slave))
            .stderr(Stdio::from_raw_fd(pty.slave))
            .pre_exec(|| {
                setsid().unwrap();
                ioctl::set_controlling(0).unwrap();
                Ok(())
            })
            .spawn()
            .and_then(|cmd_child| Ok(cmd_child))
        {
            Ok(_cmd_child) => (),
            Err(e) => {
                panic!("Failed to spawn shell: {:?}", e)
            }
        }
    }

    executor::block_on(event_loop(
        &mut input_stream,
        &mut tty_output,
        &mut pty_master_file.unwrap(),
    ));

    Ok(())
}
