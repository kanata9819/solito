use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, SlavePty};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::thread::{self, JoinHandle};

// mpsc pair
type TReader = Box<dyn Read + Send>;
type TWriter = Box<dyn Write + Send>;
// pty pair
type TMaster = Box<dyn MasterPty + Send>;
type TSlave = Box<dyn SlavePty + Send>;
// spawned child
type TChild = Box<dyn Child + Send + Sync>;

pub struct Runtime {}
impl Runtime {
    pub fn run_session(mut child: TChild, master: TMaster) {
        let (sender, receiver) = channel::<String>();
        let (reader, writer) = Self::get_reader_and_writer(master);

        // Thread to read output from the PTY.
        let _ = Self::spawn_output_thread(reader);

        // Thread to write input into the PTY.
        let tx_writer: JoinHandle<()> = Self::spawn_input_thread(receiver, writer);

        println!("You can now type commands for Bash (type 'exit' to quit):");

        // Main thread sends user input to the writer thread.
        loop {
            let mut input: String = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            if input.trim() == "exit" {
                break;
            }

            sender.send(input).unwrap();
        }

        drop(sender);
        tx_writer.join().unwrap();

        println!("Waiting for Bash to exit...");
        let status: ExitStatus = child.wait().unwrap();
        println!("Bash exited with status: {:?}", status);
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

    fn get_reader_and_writer(master: TMaster) -> (TReader, TWriter) {
        let reader: TReader = master.try_clone_reader().unwrap();
        let writer: TWriter = master.take_writer().unwrap();

        (reader, writer)
    }

    fn spawn_output_thread(mut reader: TReader) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut buffer: [u8; 1024] = [0u8; 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        println!("EOF");
                        break;
                    }
                    Ok(n) => {
                        let output: Cow<'_, str> = String::from_utf8_lossy(&buffer[..n]);
                        println!("{}", output);
                    }
                    Err(err) => {
                        eprintln!("error occured at reader.read() {}", err)
                    }
                }
            }
        })
    }

    fn spawn_input_thread(receiver: Receiver<String>, mut writer: TWriter) -> JoinHandle<()> {
        thread::spawn(move || {
            for input in receiver.iter() {
                if let Err(err) = writer.write_all(input.as_bytes()) {
                    eprintln!("Error writing to PTY: {}", err);
                    break;
                }
            }
        })
    }
}
