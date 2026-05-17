use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, SlavePty};
use std::error::Error;
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{debug, error, info};

// mpsc pair
type TReader = Box<dyn Read + Send>;
type TWriter = Box<dyn Write + Send>;
// pty pair
type TMaster = Box<dyn MasterPty + Send>;
type TSlave = Box<dyn SlavePty + Send>;
// spawned child
type TChild = Box<dyn Child + Send + Sync>;

pub(crate) struct SessionRuntime {
    child: TChild,
    input_rx: Receiver<SessionInput>,
    master: TMaster,
    output_tx: Sender<Vec<u8>>,
}

#[derive(Debug)]
pub(crate) enum SessionInput {
    Write(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

impl SessionInput {
    pub(crate) fn write(bytes: Vec<u8>) -> Self {
        Self::Write(bytes)
    }

    pub(crate) fn resize(cols: usize, rows: usize) -> Self {
        Self::Resize {
            cols: clamp_pty_size(cols),
            rows: clamp_pty_size(rows),
        }
    }
}

impl SessionRuntime {
    pub(crate) const PROCESS_NAME: &str = "nu";

    pub(crate) fn new(
        input_rx: Receiver<SessionInput>,
        output_tx: Sender<Vec<u8>>,
        cols: usize,
        rows: usize,
    ) -> Self {
        let pty_pair: PtyPair = Self::pty_pair(cols, rows);
        let child: TChild = Self::spawn_command(pty_pair.slave);

        Self {
            child,
            input_rx,
            master: pty_pair.master,
            output_tx,
        }
    }

    pub(crate) fn run_session(mut self) -> Result<(), Box<dyn Error>> {
        let reader: TReader = self.master.try_clone_reader()?;
        let writer: TWriter = self.master.take_writer()?;

        // Thread to read output from the PTY.
        Self::spawn_reading_thread(self.output_tx, reader);
        // Thread to write input and resize events into the PTY.
        Self::spawn_input_thread(self.input_rx, writer, self.master);

        let status: ExitStatus = self.child.wait()?;
        debug!("exited with status: {:?}", status);

        Ok(())
    }

    fn pty_pair(cols: usize, rows: usize) -> PtyPair {
        portable_pty::native_pty_system()
            .openpty(PtySize {
                rows: clamp_pty_size(rows),
                cols: clamp_pty_size(cols),
                pixel_height: 0,
                pixel_width: 0,
            })
            .expect("failed to create pty pair")
    }

    fn spawn_command(slave: TSlave) -> TChild {
        let cmd: CommandBuilder = CommandBuilder::new(Self::PROCESS_NAME);
        let slave: TSlave = slave;
        let child: TChild = slave.spawn_command(cmd).expect("failed to spawn command");

        child
    }

    fn spawn_reading_thread(output_tx: Sender<Vec<u8>>, mut reader: TReader) -> JoinHandle<()> {
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
                        error!("error occured at reader.read() {}", err)
                    }
                }
            }
        })
    }

    fn spawn_input_thread(
        input_rx: Receiver<SessionInput>,
        mut writer: TWriter,
        master: TMaster,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            // At first, the shell needs to know where cursor position is.
            // CSI cursor position reports are 1-based, so this means top-left.
            if let Err(err) = writer.write_all(b"\x1b[1;1R") {
                error!("initial write was failed: {}", err);
            };

            // after responed to the shell, we can communicate with it to use commands.
            while let Ok(input) = input_rx.recv() {
                match input {
                    SessionInput::Write(bytes) => {
                        if let Err(err) = writer.write_all(&bytes) {
                            error!("Error writing to PTY: {}", err);
                            break;
                        } else {
                            debug!("wrote: {}", String::from_utf8_lossy(&bytes));
                        }

                        writer.flush().expect("flush error");
                    }
                    SessionInput::Resize { cols, rows } => {
                        if let Err(err) = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_height: 0,
                            pixel_width: 0,
                        }) {
                            error!("Error resizing PTY: {}", err);
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
    value.max(1).min(u16::MAX as usize) as u16
}
