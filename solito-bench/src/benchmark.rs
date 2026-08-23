use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    cli::BenchmarkOptions,
    process_metrics::{ProcessMeasurement, measure_process},
    workload::{DURATION_ENV, FPS_ENV, MODE_ENV, READY_FILE_ENV, WORKLOAD_ENV},
};

const SOLITO_SHELL_ENV: &str = "SOLITO_SHELL_PROGRAM";

pub fn measure(options: &BenchmarkOptions) -> Result<(), Box<dyn Error>> {
    print_header(options);

    let workload = env::current_exe()?;
    let workload_duration = options
        .warmup
        .saturating_add(options.seconds)
        .saturating_add(1);
    let executable = solito_executable(options, &workload)?;
    let ready_signal = ReadySignal::new()?;
    let mut command = Command::new(&executable);

    command
        .env(WORKLOAD_ENV, "1")
        .env(MODE_ENV, options.mode.as_str())
        .env(DURATION_ENV, workload_duration.to_string())
        .env(FPS_ENV, options.fps.to_string())
        .env(READY_FILE_ENV, ready_signal.path())
        .env(SOLITO_SHELL_ENV, &workload)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    println!("Running Solito: {}", executable.display());
    let mut child = command.spawn().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to start Solito at {}: {error}",
                executable.display()
            ),
        )
    })?;
    if let Err(error) = ready_signal.wait(&mut child, Duration::from_secs(5)) {
        stop_child(&mut child);
        return Err(error.into());
    }

    let measurement_result = measure_child(&mut child, options);
    stop_child(&mut child);
    let measurement = measurement_result?;
    print_measurement(&measurement);
    Ok(())
}

fn measure_child(child: &mut Child, options: &BenchmarkOptions) -> io::Result<ProcessMeasurement> {
    let measurement = measure_process(
        child.id(),
        Duration::from_secs(options.warmup),
        Duration::from_secs(options.seconds),
    )?;
    thread::sleep(Duration::from_secs(1));
    Ok(measurement)
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn solito_executable(options: &BenchmarkOptions, workload: &Path) -> io::Result<PathBuf> {
    let candidate = options
        .solito
        .clone()
        .or_else(|| env::var_os("SOLITO_EXE").map(PathBuf::from))
        .unwrap_or_else(|| {
            workload
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(executable_name("solito"))
        });

    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Solito executable was not found at {}. Build it first or pass --solito <PATH>",
                candidate.display(),
            ),
        ))
    }
}

fn executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

fn print_header(options: &BenchmarkOptions) {
    println!(
        "Solito benchmark: mode={}, fps={}, warmup={}s, sample={}s\n",
        options.mode.as_str(),
        options.fps,
        options.warmup,
        options.seconds,
    );
}

fn print_measurement(measurement: &ProcessMeasurement) {
    println!(
        "{:<12} {:>10} {:>10} {:>12} {:>9}",
        "Terminal", "Avg CPU", "Max CPU", "Max RSS", "Samples"
    );
    println!("{}", "-".repeat(58));
    println!(
        "{:<12} {:>9.2}% {:>9.2}% {:>9.2} MiB {:>9}",
        "Solito",
        measurement.average_cpu_percent,
        measurement.maximum_cpu_percent,
        measurement.maximum_rss_bytes as f64 / 1_048_576.0,
        measurement.samples,
    );
}

struct ReadySignal {
    path: PathBuf,
}

impl ReadySignal {
    fn new() -> io::Result<Self> {
        let path = env::temp_dir().join(format!(
            "solito-bench-ready-solito-{}.txt",
            std::process::id(),
        ));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn wait(&self, child: &mut Child, timeout: Duration) -> io::Result<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if self.path.is_file() {
                return Ok(());
            }
            if let Some(status) = child.try_wait()? {
                return Err(io::Error::other(format!(
                    "terminal exited before its benchmark workload was ready: {status}"
                )));
            }
            thread::sleep(Duration::from_millis(25));
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "benchmark workload did not signal readiness within 5 seconds",
        ))
    }
}

impl Drop for ReadySignal {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::executable_name;

    #[test]
    fn executable_name_matches_the_platform() {
        let name = executable_name("solito");
        if cfg!(windows) {
            assert_eq!(name, "solito.exe");
        } else {
            assert_eq!(name, "solito");
        }
    }
}
