mod benchmark;
mod cli;
mod process_metrics;
mod workload;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if workload::requested_by_environment() {
        return workload::run_from_environment();
    }

    match cli::parse()? {
        cli::Command::Measure(options) => benchmark::measure(&options)?,
        cli::Command::Workload(config) => workload::run(&config)?,
        cli::Command::Help => print!("{}", cli::HELP),
    }

    Ok(())
}
