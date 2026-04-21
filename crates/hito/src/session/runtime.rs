use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, SlavePty};
use std::borrow::Cow;
use std::error::Error;
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{error, info};

use crate::session::parser::EscParser;

// mpsc pair
type TReader = Box<dyn Read + Send>;
type TWriter = Box<dyn Write + Send>;
// pty pair
type TMaster = Box<dyn MasterPty + Send>;
type TSlave = Box<dyn SlavePty + Send>;
// spawned child
type TChild = Box<dyn Child + Send + Sync>;

pub struct SessionRuntime {
    child: TChild,
    input_rx: Receiver<Vec<u8>>,
    output_tx: Sender<Vec<u8>>,
    master: TMaster,
    parser: EscParser,
}

impl SessionRuntime {
    pub fn new(input_rx: Receiver<Vec<u8>>, output_tx: Sender<Vec<u8>>) -> Self {
        let pty_pair: PtyPair = Self::pty_pair();
        let child: TChild = Self::spawn_command(pty_pair.slave);
        let parser: EscParser = EscParser::new();

        Self {
            child,
            input_rx,
            output_tx,
            master: pty_pair.master,
            parser,
        }
    }

    pub fn run_session(mut self) -> Result<(), Box<dyn Error>> {
        let reader: TReader = self.master.try_clone_reader()?;
        let writer: TWriter = self.master.take_writer()?;

        // Thread to read output from the PTY.
        Self::spawn_reading_thread(self.parser, reader, self.output_tx);
        // Thread to write input into the PTY.
        Self::spawn_writing_thread(self.input_rx, writer);

        let status: ExitStatus = self.child.wait()?;
        info!("exited with status: {:?}", status);

        Ok(())
    }

    fn pty_pair() -> PtyPair {
        portable_pty::native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_height: 0,
                pixel_width: 0,
            })
            .expect("failed to create pty pair")
    }

    fn spawn_command(slave: TSlave) -> TChild {
        let cmd: CommandBuilder = CommandBuilder::new("nu");
        let slave: TSlave = slave;
        let child: TChild = slave.spawn_command(cmd).expect("failed to spawn command");

        child
    }

    fn spawn_reading_thread(
        mut parser: EscParser,
        mut reader: TReader,
        output_tx: Sender<Vec<u8>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut buffer: [u8; 1024] = [0u8; 1024];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("EOF");
                        break;
                    }
                    Ok(n) => {
                        let output: Cow<'_, str> = String::from_utf8_lossy(&buffer[..n]);
                        parser.parse(output.as_bytes());
                        if let Err(err) = output_tx.send(output.as_bytes().to_vec()) {
                            error!("error occured at output_tx.send() {}", err);
                        };
                    }
                    Err(err) => {
                        error!("error occured at reader.read() {}", err)
                    }
                }
            }
        })
    }

    fn spawn_writing_thread(input_rx: Receiver<Vec<u8>>, mut writer: TWriter) -> JoinHandle<()> {
        thread::spawn(move || {
            // At first, the shell needs to know where cursor position is.
            // so I send \x1b[1;1R, it means the cursor position is now x: 0 y:0.
            if let Err(err) = writer.write_all(b"\x1b[1;1R") {
                error!("initial write was failed: {}", err);
            };

            // after responed to the shell, we can communicate with it to use commands.
            while let Ok(bytes) = input_rx.recv() {
                if let Err(err) = writer.write_all(&bytes) {
                    error!("Error writing to PTY: {}", err);
                    break;
                } else {
                    info!("wrote: {}", String::from_utf8_lossy(&bytes));
                }

                writer.flush().expect("flush error");
            }
        })
    }
}
