use std::{env, io, path::PathBuf};

use crate::workload::{WorkloadConfig, WorkloadMode};

pub const HELP: &str = r#"Solito terminal performance benchmark

Usage:
  solito-bench solito [OPTIONS]
  solito-bench compare [OPTIONS]
  solito-bench alacritty [OPTIONS]
  solito-bench workload [OPTIONS]

Options:
  --mode <full|incremental|nvim>
                              Workload type [default: full]
  --seconds <N>              Measurement duration [default: 10]
  --warmup <N>               Warmup duration [default: 2]
  --fps <N>                  Workload update rate [default: 60]
  --solito <PATH>            Solito executable override
  --alacritty <PATH>         Alacritty executable override
  -h, --help                 Print this help

Examples:
  cargo run -p solito-bench --release -- solito --mode nvim
  cargo run -p solito-bench --release -- solito --mode incremental --seconds 15
  cargo run -p solito-bench --release -- solito --seconds 10
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkTarget {
    Solito,
    Alacritty,
}

impl BenchmarkTarget {
    pub fn name(self) -> &'static str {
        match self {
            Self::Solito => "Solito",
            Self::Alacritty => "Alacritty",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkOptions {
    pub mode: WorkloadMode,
    pub seconds: u64,
    pub warmup: u64,
    pub fps: u32,
    pub solito: Option<PathBuf>,
    pub alacritty: Option<PathBuf>,
}

impl Default for BenchmarkOptions {
    fn default() -> Self {
        Self {
            mode: WorkloadMode::Full,
            seconds: 10,
            warmup: 2,
            fps: 60,
            solito: None,
            alacritty: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Compare(BenchmarkOptions),
    Measure {
        target: BenchmarkTarget,
        options: BenchmarkOptions,
    },
    Workload(WorkloadConfig),
    Help,
}

pub fn parse() -> io::Result<Command> {
    parse_from(env::args().skip(1))
}

fn parse_from(args: impl IntoIterator<Item = String>) -> io::Result<Command> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };

    if matches!(command.as_str(), "-h" | "--help" | "help") {
        return Ok(Command::Help);
    }

    let mut options = BenchmarkOptions::default();
    let mut workload_duration = None;
    let remaining = args.collect::<Vec<_>>();
    let mut index = 0;
    while index < remaining.len() {
        let option = remaining[index].as_str();
        if matches!(option, "-h" | "--help") {
            return Ok(Command::Help);
        }

        let value = remaining
            .get(index + 1)
            .ok_or_else(|| invalid(format!("missing value for {option}")))?;
        match option {
            "--mode" => options.mode = value.parse()?,
            "--seconds" => options.seconds = positive_u64(option, value)?,
            "--warmup" => {
                options.warmup = value.parse().map_err(|_| {
                    invalid(format!(
                        "{option} must be a non-negative integer, got {value}"
                    ))
                })?
            }
            "--fps" => options.fps = positive_u32(option, value)?,
            "--solito" => options.solito = Some(PathBuf::from(value)),
            "--alacritty" => options.alacritty = Some(PathBuf::from(value)),
            "--duration" => workload_duration = Some(positive_u64(option, value)?),
            _ => return Err(invalid(format!("unknown option: {option}"))),
        }
        index += 2;
    }

    match command.as_str() {
        "compare" => Ok(Command::Compare(options)),
        "solito" => Ok(Command::Measure {
            target: BenchmarkTarget::Solito,
            options,
        }),
        "alacritty" => Ok(Command::Measure {
            target: BenchmarkTarget::Alacritty,
            options,
        }),
        "workload" => Ok(Command::Workload(WorkloadConfig {
            mode: options.mode,
            duration_seconds: workload_duration.unwrap_or(options.seconds),
            fps: options.fps,
        })),
        _ => Err(invalid(format!("unknown command: {command}"))),
    }
}

fn positive_u64(option: &str, value: &str) -> io::Result<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| invalid(format!("{option} must be a positive integer, got {value}")))?;
    if parsed == 0 {
        return Err(invalid(format!("{option} must be greater than zero")));
    }
    Ok(parsed)
}

fn positive_u32(option: &str, value: &str) -> io::Result<u32> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| invalid(format!("{option} must be a positive integer, got {value}")))?;
    if parsed == 0 {
        return Err(invalid(format!("{option} must be greater than zero")));
    }
    Ok(parsed)
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::{BenchmarkTarget, Command, parse_from};
    use crate::workload::WorkloadMode;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn compare_uses_stable_defaults() {
        let command = parse_from(args(&["compare"])).unwrap();
        let Command::Compare(options) = command else {
            panic!("expected compare command");
        };

        assert_eq!(options.mode, WorkloadMode::Full);
        assert_eq!(options.seconds, 10);
        assert_eq!(options.warmup, 2);
        assert_eq!(options.fps, 60);
    }

    #[test]
    fn parses_target_and_overrides() {
        let command = parse_from(args(&[
            "solito",
            "--mode",
            "incremental",
            "--seconds",
            "15",
            "--warmup",
            "1",
            "--fps",
            "30",
        ]))
        .unwrap();
        let Command::Measure { target, options } = command else {
            panic!("expected measure command");
        };

        assert_eq!(target, BenchmarkTarget::Solito);
        assert_eq!(options.mode, WorkloadMode::Incremental);
        assert_eq!(options.seconds, 15);
        assert_eq!(options.warmup, 1);
        assert_eq!(options.fps, 30);
    }

    #[test]
    fn parses_nvim_workload_mode() {
        let command = parse_from(args(&["compare", "--mode", "nvim"])).unwrap();
        let Command::Compare(options) = command else {
            panic!("expected compare command");
        };

        assert_eq!(options.mode, WorkloadMode::Nvim);
    }

    #[test]
    fn rejects_zero_measurement_duration() {
        let error = parse_from(args(&["compare", "--seconds", "0"])).unwrap_err();

        assert!(error.to_string().contains("greater than zero"));
    }
}
