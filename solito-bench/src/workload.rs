use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

pub const WORKLOAD_ENV: &str = "SOLITO_BENCH_WORKLOAD";
pub const MODE_ENV: &str = "SOLITO_BENCH_MODE";
pub const DURATION_ENV: &str = "SOLITO_BENCH_DURATION_SECONDS";
pub const FPS_ENV: &str = "SOLITO_BENCH_FPS";
pub const READY_FILE_ENV: &str = "SOLITO_BENCH_READY_FILE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkloadMode {
    Full,
    Incremental,
    Nvim,
}

impl WorkloadMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Incremental => "incremental",
            Self::Nvim => "nvim",
        }
    }
}

impl FromStr for WorkloadMode {
    type Err = io::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "full" => Ok(Self::Full),
            "incremental" => Ok(Self::Incremental),
            "nvim" => Ok(Self::Nvim),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("mode must be full, incremental, or nvim, got {value}"),
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkloadConfig {
    pub mode: WorkloadMode,
    pub duration_seconds: u64,
    pub fps: u32,
}

pub fn requested_by_environment() -> bool {
    env::var_os(WORKLOAD_ENV).is_some_and(|value| value == "1")
}

pub fn run_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::var(MODE_ENV)
        .unwrap_or_else(|_| "full".to_string())
        .parse()?;
    let duration_seconds = parse_environment(DURATION_ENV, 13_u64)?;
    let fps = parse_environment(FPS_ENV, 60_u32)?;

    if let Some(path) = env::var_os(READY_FILE_ENV) {
        // The parent starts measurement only after the PTY child confirms that
        // the benchmark workload, rather than an idle shell, is running.
        fs::write(path, std::process::id().to_string())?;
    }

    run(&WorkloadConfig {
        mode,
        duration_seconds,
        fps,
    })?;
    Ok(())
}

fn parse_environment<T>(name: &str, fallback: T) -> io::Result<T>
where
    T: FromStr,
{
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} has an invalid value: {value}"),
            )
        }),
        Err(_) => Ok(fallback),
    }
}

pub fn run(config: &WorkloadConfig) -> io::Result<()> {
    if config.mode == WorkloadMode::Nvim {
        return run_nvim(config);
    }

    let mut output = io::stdout().lock();
    let frame_duration = Duration::from_secs_f64(1.0 / f64::from(config.fps));
    let duration = Duration::from_secs(config.duration_seconds);
    let started = Instant::now();
    let mut next_frame = started;
    let mut frame = 0_u64;

    output.write_all(b"\x1b[?1049h\x1b[?25l")?;
    if config.mode == WorkloadMode::Incremental {
        output.write_all(&full_frame(0))?;
        output.flush()?;
    }

    while started.elapsed() < duration {
        match config.mode {
            WorkloadMode::Full => output.write_all(&full_frame(frame))?,
            WorkloadMode::Incremental => output.write_all(&status_line(frame))?,
            WorkloadMode::Nvim => unreachable!("nvim uses its own workload runner"),
        }
        output.flush()?;
        frame += 1;
        next_frame += frame_duration;
        thread::sleep(next_frame.saturating_duration_since(Instant::now()));
    }

    output.write_all(b"\x1b[?25h\x1b[?1049l")?;
    output.flush()
}

fn run_nvim(config: &WorkloadConfig) -> io::Result<()> {
    let nvim = nvim_executable();
    if !nvim.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Neovim was not found at {}. Set NVIM_EXE to override it",
                nvim.display()
            ),
        ));
    }

    let script = TemporaryNvimScript::create(config)?;
    let status = Command::new(&nvim)
        .arg("--clean")
        .arg("-S")
        .arg(script.path())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "Neovim benchmark exited with {status}"
        )))
    }
}

fn nvim_executable() -> PathBuf {
    env::var_os("NVIM_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:/Program Files/Neovim/bin/nvim.exe"))
}

struct TemporaryNvimScript {
    path: PathBuf,
}

impl TemporaryNvimScript {
    fn create(config: &WorkloadConfig) -> io::Result<Self> {
        let path = env::temp_dir().join(format!("solito-bench-nvim-{}.lua", std::process::id()));
        let interval_ms = (1_000_u64 / u64::from(config.fps)).max(1);
        let duration_ms = config.duration_seconds.saturating_mul(1_000);
        let script = format!(
            r#"local uv = vim.uv or vim.loop
vim.o.termguicolors = true
vim.o.number = true
vim.o.cursorline = true
vim.o.scrolloff = 4

local lines = {{}}
for index = 1, 2000 do
  lines[index] = string.format(
    "fn render_row_%04d() {{ let frame = %06d; }} // Neovim benchmark",
    index,
    index
  )
end
vim.api.nvim_buf_set_lines(0, 0, -1, false, lines)
vim.bo.filetype = "rust"
vim.cmd("syntax enable")

local frame = 0
local update = uv.new_timer()
local finish = uv.new_timer()
_G.solito_bench_update = update
_G.solito_bench_finish = finish

update:start(0, {interval_ms}, function()
  vim.schedule(function()
    frame = frame + 1
    local line = (frame % 1950) + 1
    vim.api.nvim_win_set_cursor(0, {{ line, 0 }})
    vim.api.nvim_buf_set_lines(0, line - 1, line, false, {{
      string.format(
        "fn render_row_%04d() {{ let frame = %06d; }} // Neovim benchmark",
        line,
        frame
      )
    }})
    vim.cmd("normal! zz")
    vim.cmd("redraw!")
  end)
end)

finish:start({duration_ms}, 0, function()
  vim.schedule(function()
    update:stop()
    update:close()
    finish:stop()
    finish:close()
    vim.cmd("qa!")
  end)
end)
"#,
        );
        fs::write(&path, script)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryNvimScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn full_frame(frame: u64) -> Vec<u8> {
    let mut output = String::from("\x1b[H");
    for row in 0..24 {
        let color = 31 + row % 7;
        output.push_str(&format!(
            "\x1b[{color}m{:>3} \x1b[0mfn render_row_{row:02}() {{ let frame = {frame:06}; }} {:<40}\x1b[K\r\n",
            row + 1,
            "// Neovim-like colored grid update",
        ));
    }
    let status = status_line(frame);
    output.push_str(std::str::from_utf8(&status).expect("status line is valid UTF-8"));
    output.into_bytes()
}

fn status_line(frame: u64) -> Vec<u8> {
    format!("\x1b[25;1H\x1b[7m NORMAL  Solito benchmark  frame {frame:06} \x1b[0m\x1b[K")
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{full_frame, status_line};

    #[test]
    fn full_frame_crosses_the_pty_read_chunk_boundary() {
        assert!(full_frame(1).len() > 1024);
    }

    #[test]
    fn incremental_update_stays_within_one_pty_read_chunk() {
        assert!(status_line(1).len() < 1024);
    }
}
