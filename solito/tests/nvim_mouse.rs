//! Optional real-PTY check: cargo test -p solito --test nvim_mouse -- --ignored
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
#[ignore = "requires nvim on PATH and a native PTY"]
fn nvim_receives_click_drag_and_scroll() {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new("nvim");
    command.args(["--clean", "-n", "-i", "NONE", "-c", "language messages C", "-c",
        "set mouse=a mousetime=0 showmode noruler noshowcmd laststatus=0", "-c",
        "lua vim.api.nvim_buf_set_lines(0,0,-1,false,vim.tbl_map(function(i) return string.format('row %03d abcdefghijklmnopqrstuvwxyz',i) end,vim.fn.range(1,200)))"]);
    let _child = ChildGuard(pair.slave.spawn_command(command).unwrap());
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = [0; 16384];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 || tx.send(buffer[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let mut terminal = TerminalState::new(TerminalSize::new(80, 24));
    let mut settle = |writer: &mut Box<dyn Write + Send>| {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(bytes) => {
                    if bytes.windows(4).any(|part| part == b"\x1b[6n") {
                        writer.write_all(b"\x1b[1;1R").unwrap();
                        writer.flush().unwrap();
                    }
                    terminal.apply_terminal_output(&bytes);
                }
                Err(_) => break,
            }
        }
        (terminal.snapshot(), terminal.mouse_mode())
    };
    let (_, mode) = settle(&mut writer);
    // The reports are produced by Solito's encoder, not hard-coded input strings.
    assert_ne!(mode.tracking, 0, "Nvim did not enable mouse tracking");
    writer.write_all(b"\x1b").unwrap();
    writer
        .write_all(&mode.encode(0, false, 5, 4).unwrap())
        .unwrap();
    writer
        .write_all(&mode.encode(0, true, 5, 4).unwrap())
        .unwrap();
    writer.flush().unwrap();
    let (clicked, _) = settle(&mut writer);
    assert_eq!(
        (clicked.cursor_col, clicked.cursor_row),
        (5, 4),
        "click did not move Nvim's cursor"
    );
    writer
        .write_all(&mode.encode(0, false, 5, 4).unwrap())
        .unwrap();
    writer.flush().unwrap();
    settle(&mut writer);
    writer
        .write_all(&mode.encode(32, false, 10, 6).unwrap())
        .unwrap();
    writer.flush().unwrap();
    settle(&mut writer);
    writer
        .write_all(&mode.encode(0, true, 10, 6).unwrap())
        .unwrap();
    writer.flush().unwrap();
    let (dragged, _) = settle(&mut writer);
    assert_eq!((dragged.cursor_col, dragged.cursor_row), (10, 6));
    assert!(dragged.lines.iter().any(|line| {
        line.iter()
            .map(|c| c.ch)
            .collect::<String>()
            .contains("VISUAL")
    }));
    writer.write_all(b"\x1b").unwrap();
    for _ in 0..5 {
        writer
            .write_all(&mode.encode(65, false, 10, 6).unwrap())
            .unwrap();
    }
    writer.flush().unwrap();
    let (scrolled, _) = settle(&mut writer);
    assert_ne!(
        clicked.lines[0], scrolled.lines[0],
        "wheel did not scroll Nvim"
    );
}
