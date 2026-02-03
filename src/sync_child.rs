use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};

use anyhow::Context;

pub struct ChildHandle {
    pub stdin_handle: Option<JoinHandle<()>>,
    pub stderr_handle: Option<JoinHandle<()>>,
    pub stdout_handle: Option<JoinHandle<()>>,
    child: Child,
}

impl ChildHandle {
    pub fn join(&mut self) -> anyhow::Result<ExitStatus> {
        let _ = self.stdin_handle.take();
        for handle in [self.stdout_handle.take(), self.stderr_handle.take()] {
            if let Some(handle) = handle {
                let _ = handle.join();
            }
        }

        Ok(self.child.wait()?)
    }
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub fn spawn_child_process(
    args: &[String],
    stdout_tx: Option<Sender<String>>,
    stderr_tx: Option<Sender<String>>,
    stdin_rx: Option<Receiver<u8>>,
) -> anyhow::Result<ChildHandle> {
    let mut iter = args.iter();

    let mut cmd = Command::new(iter.next().context("Arguments aren't enough")?);
    cmd.args(iter)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let mut stdin_handle = None;
    let mut stderr_handle = None;
    let mut stdout_handle = None;

    if let (Some(stdout_tx), Some(stdout)) = (stdout_tx, child.stdout.take()) {
        stdout_handle = Some(std::thread::spawn(|| read_lines(stdout, stdout_tx)));
    }
    if let (Some(stderr_tx), Some(stderr)) = (stderr_tx, child.stderr.take()) {
        stderr_handle = Some(std::thread::spawn(|| read_lines(stderr, stderr_tx)));
    }
    if let (Some(stdin_rx), Some(stdin)) = (stdin_rx, child.stdin.take()) {
        stdin_handle = Some(std::thread::spawn(|| write_bytes(stdin, stdin_rx)));
    }

    // let stdout = child.stdout.take().context("No stdout of child")?;
    // let stderr = child.stderr.take().context("No stderr of child")?;
    // let stdin = child.stdin.take().context("No stdin of child")?;
    Ok(ChildHandle {
        child,
        stdin_handle,
        stderr_handle,
        stdout_handle,
    })
}

fn read_lines<T>(reader: T, sender: Sender<String>)
where
    T: Read + Unpin,
{
    let mut reader = BufReader::new(reader);
    loop {
        let mut buf = String::new();
        if let Err(err) = reader.read_line(&mut buf) {
            log::error!("failed to read line from child {}", err);
            break;
        } else {
            if buf.is_empty() {
                log::info!("received child stdout eof");
                break;
            }

            if buf.ends_with('\n') {
                buf.pop();
            }

            if let Err(err) = sender.send(buf) {
                log::error!("failed to send line from child {}", err);
                break;
            }
        }
    }
}

fn write_bytes<T>(mut writer: T, receiver: Receiver<u8>)
where
    T: Write + Unpin,
{
    while let Ok(msg) = receiver.recv() {
        if let Err(err) = writer.write(&[msg]) {
            log::error!("failed to write to stdin of child {}", err);
            break;
        }
    }
}

// fn start_child(
//     args: Vec<String>,
//     stdout_sender: Option<Sender<String>>,
//     stderr_sender: Option<Sender<String>>,
//     stdin_rx: Option<Receiver<u8>>,
// ) -> anyhow::Result<ChildHandle> {
//     log::info!("Starting child process with args {:?}", args);
//     // let (stdin_tx, stdin_rx) = std::sync::mpsc::channel();
//     let child_handle = spawn_child_process(&args, stdout_sender, stderr_sender, stdin_rx)?;

//     Ok(child_handle)
// }
