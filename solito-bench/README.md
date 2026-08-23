# solito-bench

`solito-bench` runs deterministic ANSI workloads or a real scripted Neovim session inside Solito, then reports process CPU and memory use. It never rewrites the user's Solito configuration; Solito receives the benchmark program through the temporary `SOLITO_SHELL_PROGRAM` environment override.

## Prepare release binaries

From the Solito workspace:

```bash
cargo build --release -p solito -p solito-bench
```

## Run

Measure Solito with the full-screen workload:

```bash
cargo run -p solito-bench --release -- solito
```

Measure small incremental updates:

```bash
cargo run -p solito-bench --release -- solito --mode incremental
```

Run a real Neovim instance that continuously edits, scrolls, syntax-highlights, and redraws a Rust buffer:

```bash
cargo run -p solito-bench --release -- solito --mode nvim
```

The root `justfile` provides the same Neovim measurement with a release build first:

```bash
just bench       # 10-second sample
just bench 15    # 15-second sample
```

The Neovim mode uses `C:/Program Files/Neovim/bin/nvim.exe` by default. Override it with the `NVIM_EXE` environment variable.

Measure only Solito for 15 seconds:

```bash
cargo run -p solito-bench --release -- solito --seconds 15
```

Use an explicit Solito executable path when the default does not apply:

```bash
cargo run -p solito-bench --release -- solito \
  --solito C:/path/to/solito.exe
```

CPU percentages follow the process convention where 100% is one fully occupied logical CPU. `full` repaints a 25-row colored terminal grid every frame; `incremental` draws the grid once and updates only the status line; `nvim` runs a real automated Neovim session.
