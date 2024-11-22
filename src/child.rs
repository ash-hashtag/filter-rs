use std::process::{ExitStatus, Stdio};

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
    pub result_handle: JoinHandle<std::io::Result<ExitStatus>>,
    pub stdin_handle: JoinHandle<()>,
    pub stderr_handle: JoinHandle<()>,
    pub stdout_handle: JoinHandle<()>,
}

// impl Drop for ChildHandle {
//     fn drop(&mut self) {
//         self.result_handle.abort();
//         self.stdin_handle.abort();
//         self.stdout_handle.abort();
//         self.stderr_handle.abort();
//     }
// }

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
        result_handle: tokio::spawn(get_result(child)),
        stdin_handle: tokio::spawn(write_bytes(stdin, stdin_rx)),
        stderr_handle: tokio::spawn(read_lines(stderr, stderr_tx)),
        stdout_handle: tokio::spawn(read_lines(stdout, stdout_tx)),
    })
}

async fn get_result(mut child: Child) -> std::io::Result<ExitStatus> {
    child.wait().await
}

async fn read_lines<T>(reader: T, sender: Sender)
where
    T: AsyncRead + AsyncReadExt + Unpin,
{
    let mut reader = BufReader::new(reader);
    loop {
        let mut buf = String::new();
        if let Err(err) = reader.read_line(&mut buf).await {
            eprintln!("failed to read line from child {}", err);
            break;
        } else {
            if let Err(err) = sender.send(buf) {
                eprintln!("failed to send line from child {}", err);
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
            eprintln!("failed to write to stdin of child {}", err);
            break;
        }
    }
}
