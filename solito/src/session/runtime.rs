//! PTY process lifecycle and byte transport.
//!
//! This module never interprets terminal output; `solito-terminal` owns that job.

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, SlavePty};
use solito_terminal::TerminalSize;
use std::error::Error;
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{debug, error, info};

type PtyReader = Box<dyn Read + Send>;
type PtyWriter = Box<dyn Write + Send>;
type PtyMaster = Box<dyn MasterPty + Send>;
type PtySlave = Box<dyn SlavePty + Send>;
type PtyChild = Box<dyn Child + Send + Sync>;

pub(crate) struct SessionRuntime {
    child: PtyChild,
    input_rx: Receiver<SessionInput>,
    master: PtyMaster,
    output_tx: Sender<Vec<u8>>,
}

#[derive(Debug)]
pub(crate) enum SessionInput {
    Write(Vec<u8>),
    Resize(TerminalSize),
}

impl SessionInput {
    pub(crate) fn write(bytes: Vec<u8>) -> Self {
        Self::Write(bytes)
    }

    pub(crate) fn resize(size: TerminalSize) -> Self {
        Self::Resize(size)
    }
}

impl SessionRuntime {
    pub(crate) fn new(
        input_rx: Receiver<SessionInput>,
        output_tx: Sender<Vec<u8>>,
        size: TerminalSize,
        shell_program: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let pty_pair = Self::pty_pair(size)?;
        let child = Self::spawn_command(&pty_pair.slave, shell_program)?;

        Ok(Self {
            child,
            input_rx,
            master: pty_pair.master,
            output_tx,
        })
    }

    pub(crate) fn run_session(mut self) -> Result<(), Box<dyn Error>> {
        let reader: PtyReader = self.master.try_clone_reader()?;
        let writer: PtyWriter = self.master.take_writer()?;

        // Thread to read output from the PTY.
        Self::spawn_reading_thread(self.output_tx, reader);
        // Thread to write input and resize events into the PTY.
        Self::spawn_input_thread(self.input_rx, writer, self.master);

        let status: ExitStatus = self.child.wait()?;
        debug!("exited with status: {:?}", status);

        Ok(())
    }

    fn pty_pair(size: TerminalSize) -> Result<PtyPair, Box<dyn Error>> {
        Ok(portable_pty::native_pty_system().openpty(PtySize {
            rows: clamp_pty_size(size.rows),
            cols: clamp_pty_size(size.cols),
            pixel_height: 0,
            pixel_width: 0,
        })?)
    }

    fn spawn_command(slave: &PtySlave, shell_program: &str) -> Result<PtyChild, Box<dyn Error>> {
        let cmd: CommandBuilder = CommandBuilder::new(shell_program);
        Ok(slave.spawn_command(cmd)?)
    }

    fn spawn_reading_thread(output_tx: Sender<Vec<u8>>, mut reader: PtyReader) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut buffer: [u8; 1024] = [0u8; 1024];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("EOF");
                        break;
                    }
                    Ok(n) => {
                        if output_tx.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        error!("failed to read PTY output: {err}");
                        break;
                    }
                }
            }
        })
    }

    fn spawn_input_thread(
        input_rx: Receiver<SessionInput>,
        mut writer: PtyWriter,
        master: PtyMaster,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            // The shell first needs to know the cursor position.
            // CSI cursor position reports are 1-based, so this means top-left.
            if let Err(err) = writer.write_all(b"\x1b[1;1R") {
                error!("failed to report initial cursor position: {err}");
                return;
            }

            // After that response, normal input and resize events can be forwarded.
            while let Ok(input) = input_rx.recv() {
                match input {
                    SessionInput::Write(bytes) => {
                        if let Err(err) = writer.write_all(&bytes) {
                            error!("failed to write PTY input: {err}");
                            break;
                        }
                        debug!("wrote: {}", String::from_utf8_lossy(&bytes));

                        if let Err(err) = writer.flush() {
                            error!("failed to flush PTY input: {err}");
                            break;
                        }
                    }
                    SessionInput::Resize(size) => {
                        let cols = clamp_pty_size(size.cols);
                        let rows = clamp_pty_size(size.rows);
                        if let Err(err) = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_height: 0,
                            pixel_width: 0,
                        }) {
                            error!("failed to resize PTY: {err}");
                        } else {
                            debug!("resized PTY: cols={}, rows={}", cols, rows);
                        }
                    }
                }
            }
        })
    }
}

fn clamp_pty_size(value: usize) -> u16 {
    u16::try_from(value.clamp(1, usize::from(u16::MAX))).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::clamp_pty_size;

    #[test]
    fn pty_size_stays_within_valid_range() {
        assert_eq!(clamp_pty_size(0), 1);
        assert_eq!(clamp_pty_size(80), 80);
        assert_eq!(clamp_pty_size(usize::MAX), u16::MAX);
    }
}
