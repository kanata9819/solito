//! Run with: cargo test -p solito --test shell_startup -- --ignored --nocapture
use portable_pty::{CommandBuilder, PtySize};
use solito_terminal::{TerminalSize, TerminalState};
use std::{
    io::{Read, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

struct ChildGuard(Box<dyn portable_pty::Child + Send + Sync>);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires Nushell on PATH and a native PTY"]
fn nushell_displays_initial_prompt_without_keyboard_input() {
    for _ in 0..20 {
        let pair = portable_pty::native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let program = std::env::var_os("SOLITO_SHELL_PROGRAM").unwrap_or_else(|| "nu".into());
        let mut command = CommandBuilder::new(program);
        command.args([
            "--no-config-file",
            "--interactive",
            "--execute",
            "$env.PROMPT_COMMAND = {|| 'SOLITO_READY' }; $env.PROMPT_COMMAND_RIGHT = {|| '' }",
        ]);
        let _child = ChildGuard(pair.slave.spawn_command(command).unwrap());
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = [0; 16384];
            while let Ok(n) = reader.read(&mut bytes) {
                if n == 0 || tx.send(bytes[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut terminal = TerminalState::new(TerminalSize::new(80, 24));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let bytes = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("Nushell did not display its initial prompt");
            terminal.apply_terminal_output(&bytes);
            let responses = terminal.take_responses();
            if !responses.is_empty() {
                writer.write_all(&responses).unwrap();
                writer.flush().unwrap();
            }
            if terminal.snapshot().lines.iter().any(|line| {
                line.iter()
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .contains("SOLITO_READY")
            }) {
                break;
            }
            assert!(Instant::now() < deadline, "Nushell startup timed out");
        }
    }
}
