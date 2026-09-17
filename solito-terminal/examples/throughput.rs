//! Deterministic CPU benchmark: cargo run --release -p solito-terminal --example throughput
use solito_terminal::{TerminalSize, TerminalState};
use std::{hint::black_box, time::Instant};

fn measure(name: &str, setup: &[u8], inputs: [&[u8]; 2], iterations: usize, snapshot: bool) {
    let mut samples = Vec::new();
    for _ in 0..5 {
        let mut terminal = TerminalState::new(TerminalSize::new(120, 40));
        terminal.apply_terminal_output(setup);
        let mut previous = terminal.snapshot();
        let started = Instant::now();
        for iteration in 0..iterations {
            terminal.apply_terminal_output(black_box(inputs[iteration % 2]));
            if snapshot {
                previous = black_box(terminal.snapshot());
            }
        }
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        black_box(previous);
        black_box(terminal.snapshot());
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{name:24} {:9.2} ms (median of 5, {iterations} iterations)",
        samples[2]
    );
}

fn main() {
    let line = format!("{}\r\n", "0123456789abcdef".repeat(7));
    let history = line.repeat(10_050);
    let repaint = (1..=40)
        .map(|row| {
            format!(
                "\x1b[{row};1H\x1b[38;2;100;150;200m{}",
                "abcdefghij".repeat(12)
            )
        })
        .collect::<String>();
    let repaint_next = repaint.replace("abcdefghij", "ABCDEFGHIJ");
    measure(
        "full repaint + snapshot",
        b"\x1b[?1049h",
        [repaint.as_bytes(), repaint_next.as_bytes()],
        2_000,
        true,
    );
    measure(
        "scroll at history limit",
        history.as_bytes(),
        [line.as_bytes(); 2],
        30_000,
        false,
    );
    measure(
        "edit with 10k history",
        history.as_bytes(),
        [b"\r\x1b[32mstatus0\x1b[0m", b"\r\x1b[32mstatus1\x1b[0m"],
        10_000,
        true,
    );
    measure(
        "cursor control",
        b"",
        [b"\x1b[1;1H\x1b[2C\x1b[1D"; 2],
        500_000,
        false,
    );
}
