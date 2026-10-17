//! Run with: cargo test -p solito --test resize -- --ignored --nocapture
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
fn nvim_resize_keeps_text_and_cursor_aligned() {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new("nvim");
    command.args([
        "--clean", "-n", "-i", "NONE", "-c", "language messages C", "-c",
        "set nowrap noruler noshowcmd laststatus=0", "-c",
        "lua vim.api.nvim_buf_set_lines(0,0,-1,false,vim.tbl_map(function(i) return string.format('row %03d abcdefghijklmnopqrstuvwxyz',i) end,vim.fn.range(1,200))); vim.api.nvim_win_set_cursor(0,{5,8})",
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
    let mut settle = |terminal: &mut TerminalState| {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut received = false;
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(bytes) => {
                    received = true;
                    terminal.apply_terminal_output(&bytes);
                    let responses = terminal.take_responses();
                    if !responses.is_empty() {
                        writer.write_all(&responses).unwrap();
                        writer.flush().unwrap();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) if received => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(err) => panic!("PTY output stopped: {err}"),
            }
        }
        panic!("Nvim did not finish redrawing");
    };
    settle(&mut terminal);
    for (cols, rows) in [(40, 12), (100, 32), (20, 8), (80, 24)] {
        terminal.resize(TerminalSize::new(cols, rows));
        pair.master
            .resize(PtySize {
                cols: cols as u16,
                rows: rows as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        settle(&mut terminal);
        let snapshot = terminal.snapshot();
        assert_eq!(snapshot.lines.len(), rows);
        let text = |row: usize| {
            snapshot.lines[row]
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>()
        };
        assert_eq!(snapshot.cursor_col, 8);
        assert!(text(snapshot.cursor_row).starts_with("row 005 "));
        // Nvim changes the viewport on resize to retain the cursor's relative height.
        let first: usize = text(0)[4..7].parse().unwrap();
        for row in 0..rows - 1 {
            let actual = text(row);
            let expected = format!("row {:03} abcdefghijklmnopqrstuvwxyz", first + row);
            let expected = &expected[..expected.len().min(cols)];
            assert_eq!(actual.trim_end(), expected, "{cols}x{rows}, row {row}");
        }
    }
}
