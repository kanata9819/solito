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

## Terminal CPU benchmark

```bash
cargo run --release --offline -p solito-terminal --example throughput
```

This excludes the renderer, GPU, and PTY. Each result is the median of five samples;
snapshot workloads keep the previous snapshot alive to exercise copy-on-write.
For a before/after comparison, use the same example in both source trees and
separate Cargo target directories so artifacts from different revisions cannot mix.

Measured on the same Windows machine, before (`bfb0308`) versus the optimized code:

| Workload | Iterations | Before | After | Speedup |
| --- | ---: | ---: | ---: | ---: |
| Full repaint + snapshot | 2,000 | 111.62 ms | 103.25 ms | 1.08x |
| Scroll at history limit | 30,000 | 62.80 ms | 46.35 ms | 1.35x |
| Edit + snapshot with 10k history | 10,000 | 464.45 ms | 10.74 ms | 43.24x |
| Cursor control | 500,000 | 135.11 ms | 30.91 ms | 4.37x |

These are component timings, not whole-application frame-rate improvements.

The full application was also compared on 2026-09-19: three runs per version,
60 updates/second, 5-second warmup and 20-second samples, in before/after,
after/before, before/after order. Average CPU samples were 18.67%, 11.33%,
20.00% before and 12.27%, 11.09%, 20.23% after. The overlapping ranges and
large run-to-run variation do not establish an application-wide CPU improvement
or a consistent regression. Configuration was unchanged (1000x600, Cascadia
Mono 17, line height 20, no backdrop).

Validation commands:

```bash
cargo test --workspace --offline
cargo test -p solito-renderer --offline gpu_text_updates -- --ignored
cargo test -p solito --offline --test nvim_mouse -- --ignored
```

The explicit ignored tests require a GPU adapter and native Neovim/PTY,
respectively. They check layout/invalidation and input integration; they do not
replace visual inspection of the window.

## Startup time (Windows)

```powershell
./solito-bench/startup.ps1 -Executable ./target/release/solito.exe -Runs 5
```

Measures process launch to the visible `Solito` window, after its first draw.
The script uses `cmd.exe` through the existing shell environment override, leaves
the configuration file unchanged, and terminates only the processes it starts.
It excludes shell prompt readiness and does not flush OS or driver caches.

On 2026-09-20, five release launches on the same machine gave a median of
1,061.00 ms before and 621.80 ms after (41.4% less startup time).
The changes remove duplicate font discovery, overlap font discovery with GPU
initialization, and use DirectX 12 on Windows instead of initializing Vulkan too.
Other platforms retain the previous backend selection. For driver compatibility
testing on Windows, set `$env:WGPU_BACKEND = 'vulkan'` before launching Solito;
remove the variable to restore the default.
