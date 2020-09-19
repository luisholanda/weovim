use super::events::*;
use super::msg;
use bumpalo::Bump;
use std::{future::Future, io, process::Stdio, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

const MEM_ARENA_INITIAL_CAPACITY: usize = 2 * 1024;
const RAW_IO_BUF_INITIAL_CAPACITY: usize = 16 * 1024;

/// A type that can react to neovim events.
///
/// Currently, only redraw events are handled.
pub trait EventListener: Send + 'static {
    /// A redraw event was received.
    fn on_redraw_event<'e>(&mut self, event: RedrawEvent<'e>);
}

/// A [`EventListener`] that logs every event received.
pub struct LoggerEventListener;

impl EventListener for LoggerEventListener {
    fn on_redraw_event<'e>(&mut self, event: RedrawEvent<'e>) {
        log::debug!("Received redraw event: {:?}", event)
    }
}

pub(super) struct RpcProcess {
    stdin: ChildStdin,
    rpc_buf: Vec<u8>,
    msg_id_counter: u32,
}

impl RpcProcess {
    pub(super) fn spawn() -> io::Result<(Self, EventReceiver)> {
        let (stdin, stdout) = nvim_process()?;

        let rpc = Self {
            stdin,
            msg_id_counter: 0,
            rpc_buf: Vec::with_capacity(RAW_IO_BUF_INITIAL_CAPACITY),
        };

        let recv = EventReceiver {
            stdout,
            mem_arena: Bump::with_capacity(MEM_ARENA_INITIAL_CAPACITY),
        };

        Ok((rpc, recv))
    }

    pub(super) fn rpc_method<'p, 'm>(
        &'p mut self,
        method: &'m str,
        n_args: u32,
    ) -> RpcMethod<'p, 'm> {
        let id = self.msg_id_counter;
        self.msg_id_counter += 1;
        if self.msg_id_counter == std::u32::MAX {
            self.msg_id_counter = 0;
        }

        self.create_rpc_method(id, method, n_args)
    }

    pub(crate) fn rpc_method_forget<'p, 'm>(
        &'p mut self,
        method: &'m str,
        n_args: u32,
    ) -> RpcMethod<'p, 'm> {
        self.create_rpc_method(std::u32::MAX, method, n_args)
    }

    fn create_rpc_method<'p, 'm>(
        &'p mut self,
        id: u32,
        method: &'m str,
        n_args: u32,
    ) -> RpcMethod<'p, 'm> {
        self.rpc_buf.clear();
        {
            let _ = rmp::encode::write_array_len(&mut self.rpc_buf, 4);
            let _ = rmp::encode::write_uint(&mut self.rpc_buf, 0);
            let _ = rmp::encode::write_uint(&mut self.rpc_buf, id as u64);
            let _ = rmp::encode::write_str(&mut self.rpc_buf, method);
            let _ = rmp::encode::write_array_len(&mut self.rpc_buf, n_args);
        }

        RpcMethod {
            stdin: &mut self.stdin,
            method,
            buf: &mut self.rpc_buf,
        }
    }
}

pub(super) struct RpcMethod<'p, 'm> {
    stdin: &'p mut ChildStdin,
    method: &'m str,
    buf: &'p mut Vec<u8>,
}

impl RpcMethod<'_, '_> {
    pub(super) fn add_str_arg(&mut self, arg: &str) {
        let _ = rmp::encode::write_str(&mut self.buf, arg);
    }

    pub(super) fn add_u64_arg(&mut self, arg: u64) {
        let _ = rmp::encode::write_uint(&mut self.buf, arg);
    }

    pub(super) fn add_i64_arg(&mut self, arg: i64) {
        let _ = rmp::encode::write_sint(&mut self.buf, arg);
    }

    pub(super) fn add_bool_arg(&mut self, arg: bool) {
        let _ = rmp::encode::write_bool(&mut self.buf, arg);
    }

    pub(super) fn start_array_arg(&mut self, len: u32) {
        let _ = rmp::encode::write_array_len(&mut self.buf, len);
    }

    pub(super) fn start_map_arg(&mut self, n_pairs: u32) {
        let _ = rmp::encode::write_map_len(&mut self.buf, n_pairs);
    }

    pub(super) fn add_str_pair(&mut self, key: &str, arg: &str) {
        self.add_str_arg(key);
        let _ = rmp::encode::write_str(&mut self.buf, arg);
    }

    pub(super) fn add_u64_pair(&mut self, key: &str, arg: u64) {
        self.add_str_arg(key);
        let _ = rmp::encode::write_uint(&mut self.buf, arg);
    }

    pub(super) fn add_i64_pair(&mut self, key: &str, arg: i64) {
        self.add_str_arg(key);
        let _ = rmp::encode::write_sint(&mut self.buf, arg);
    }

    pub(super) fn add_bool_pair(&mut self, key: &str, arg: bool) {
        self.add_str_arg(key);
        let _ = rmp::encode::write_bool(&mut self.buf, arg);
    }

    pub(super) async fn send(self) -> io::Result<()> {
        log::debug!(
            "Sending RPC method '{}', total payload length: {}",
            self.method,
            self.buf.len()
        );
        log::trace!("payload: {:?}", self.buf);
        self.stdin.write_all(&self.buf).await?;
        self.stdin.flush().await?;
        log::trace!("RPC method sent");

        Ok(())
    }
}

pub(super) struct EventReceiver {
    stdout: ChildStdout,
    mem_arena: Bump,
}

impl EventReceiver {
    pub(super) async fn start_loop<L: EventListener>(mut self, mut listener: L) -> io::Result<!> {
        let mut raw_buf = std::vec::Vec::with_capacity(RAW_IO_BUF_INITIAL_CAPACITY);
        loop {
            raw_buf.clear();
            let n = self.stdout.read_buf(&mut raw_buf).await?;
            if n == 0 {
                std::sync::atomic::spin_loop_hint();
                continue;
            }

            log::trace!("Read {} bytes from stdout", n);

            self.mem_arena.reset();
            let mut recv = &raw_buf[..n];
            while !recv.is_empty() {
                let _ = msg::read_array_len(&mut recv)?;
                match msg::read_u64(&mut recv)? {
                    // request
                    0 => {
                        log::error!("received request from neovim process");
                        recv = &[];
                    }
                    // responses
                    1 => {
                        log::warn!("received response from neovim process");
                        recv = &[];
                    }
                    // notifications
                    2 => {
                        match msg::read_string(&mut recv)? {
                            "redraw" => match RedrawEvent::decode(&mut recv, &self.mem_arena) {
                                Ok(events) => {
                                    events.into_iter().for_each(|e| listener.on_redraw_event(e))
                                }
                                Err(error) => {
                                    log::error!("Error while decoding RPC message: {}", error);
                                    recv = &[];
                                }
                            },
                            not => {
                                log::warn!("received unknown notification type '{}'", not);
                                recv = &[];
                            }
                        };
                    }
                    _ => unreachable!("received invalid RPC type"),
                }
            }
        }
    }
}

fn nvim_process() -> io::Result<(ChildStdin, ChildStdout)> {
    let mut nvim = Command::new("nvim")
        .arg("--embed")
        .current_dir(std::env::current_dir()?)
        .envs(std::env::vars())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    log::info!("Spawned neovim process at PID {}", nvim.id());

    let stdin = nvim
        .stdin()
        .take()
        .expect("child neovim process stdin not configured");

    let stdout = nvim
        .stdout()
        .take()
        .expect("child neovim process stdout not configured");

    tokio::spawn(async move {
        log::info!("Waiting for neovim process to finish");
        match nvim.await {
            Ok(status) => log::error!("neovim process exited with status {}", status),
            Err(error) => log::error!("neovim process exited with error: {}", error),
        }

        std::process::exit(1);
    });

    Ok((stdin, stdout))
}
