use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, SlavePty};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{error, info};

// mpsc pair
type TReader = Box<dyn Read + Send>;
type TWriter = Box<dyn Write + Send>;
// pty pair
type TMaster = Box<dyn MasterPty + Send>;
type TSlave = Box<dyn SlavePty + Send>;
// spawned child
type TChild = Box<dyn Child + Send + Sync>;

pub struct Runtime {
    child: TChild,
    input_tx: Sender<String>,
    input_rx: Receiver<String>,
    master: TMaster,
}

impl Runtime {
    pub fn new() -> Self {
        let pty_pair: PtyPair = Self::pty_pair();
        let child: TChild = Self::spawn_command(pty_pair.slave);
        let (input_tx, input_rx) = channel::<String>();

        Self {
            child,
            input_tx,
            input_rx,
            master: pty_pair.master,
        }
    }

    pub fn run_session(mut self) {
        let reader: TReader = self.master.try_clone_reader().unwrap();
        let writer: TWriter = self.master.take_writer().unwrap();

        // Thread to read output from the PTY.
        Self::spawn_reading_thread(reader);
        // Thread to write input into the PTY.
        Self::spawn_writing_thread(self.input_rx, writer);

        info!("You can now type commands for Bash (type 'exit' to quit):");

        // Main thread sends user input to the writer thread.
        loop {
            let mut input: String = String::new();
            if let Err(err) = std::io::stdin().read_line(&mut input) {
                error!("read line error: {}", err);
            };

            if input.trim() == "exit" {
                break;
            } else if input.is_empty() {
                continue;
            }

            if let Err(err) = self.input_tx.send(input) {
                error!("send error occured: {}", err);
                break;
            };
        }

        drop(self.input_tx);

        info!("Waiting for Bash to exit...");
        let status: ExitStatus = self.child.wait().unwrap();
        info!("Bash exited with status: {:?}", status);
    }

    pub fn pty_pair() -> PtyPair {
        portable_pty::native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_height: 0,
                pixel_width: 0,
            })
            .expect("failed to create pty pair")
    }

    pub fn spawn_command(slave: TSlave) -> TChild {
        let cmd: CommandBuilder = CommandBuilder::new("nu");
        let slave: TSlave = slave;
        let child: TChild = slave.spawn_command(cmd).expect("failed to spawn command");

        child
    }

    fn spawn_reading_thread(mut reader: TReader) -> JoinHandle<()> {
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
                        info!("{}", output);
                    }
                    Err(err) => {
                        error!("error occured at reader.read() {}", err)
                    }
                }
            }
        })
    }

    fn spawn_writing_thread(input_rx: Receiver<String>, mut writer: TWriter) -> JoinHandle<()> {
        thread::spawn(move || {
            // At first, the shell needs to know where cursor position is.
            // so I send \x1b[1;1R, it means the cursor position is now x: 0 y:0.
            if let Err(err) = writer.write_all(b"\x1b[1;1R") {
                error!("initial write was failed: {}", err);
            };

            // after responed to the shell, we can communicate with it to use commands.
            while let Ok(str) = input_rx.recv() {
                if let Err(err) = writer.write_all(&str.as_bytes()) {
                    error!("Error writing to PTY: {}", err);
                    break;
                } else {
                    info!("wrote: {}", str);
                }

                writer.flush().expect("flush error");
            }
        })
    }
}
