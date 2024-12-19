use std::process::Stdio;

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use anyhow::Context;

pub type Sender = UnboundedSender<String>;
pub type Receiver = UnboundedReceiver<u8>;

pub struct ChildHandle {
    pub stdin_handle: JoinHandle<()>,
    pub stderr_handle: JoinHandle<()>,
    pub stdout_handle: JoinHandle<()>,
    child: Child,
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        self.stdin_handle.abort();
        self.stdout_handle.abort();
        self.stderr_handle.abort();
        self.child.start_kill().unwrap();
    }
}

pub fn spawn_child_process(
    args: &[String],
    stdout_tx: Sender,
    stderr_tx: Sender,
    stdin_rx: Receiver,
) -> anyhow::Result<ChildHandle> {
    let mut iter = args.iter();

    let mut cmd = Command::new(iter.next().context("Arguments aren't enough")?);
    cmd.args(iter)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().context("No stdout of child")?;
    let stderr = child.stderr.take().context("No stderr of child")?;
    let stdin = child.stdin.take().context("No stdin of child")?;

    Ok(ChildHandle {
        child,
        stdin_handle: tokio::spawn(write_bytes(stdin, stdin_rx)),
        stderr_handle: tokio::spawn(read_lines(stderr, stderr_tx)),
        stdout_handle: tokio::spawn(read_lines(stdout, stdout_tx)),
    })
}

async fn read_lines<T>(reader: T, sender: Sender)
where
    T: AsyncRead + AsyncReadExt + Unpin,
{
    let mut reader = BufReader::new(reader);
    loop {
        let mut buf = String::new();
        if let Err(err) = reader.read_line(&mut buf).await {
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

async fn write_bytes<T>(mut writer: T, mut receiver: Receiver)
where
    T: AsyncWrite + AsyncWriteExt + Unpin,
{
    while let Some(msg) = receiver.recv().await {
        if let Err(err) = writer.write_u8(msg).await {
            log::error!("failed to write to stdin of child {}", err);
            break;
        }
    }
}
