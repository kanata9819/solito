use std::{io, time::Duration};

#[derive(Clone, Debug)]
pub struct ProcessMeasurement {
    pub average_cpu_percent: f64,
    pub maximum_cpu_percent: f64,
    pub maximum_rss_bytes: u64,
    pub samples: usize,
}

#[cfg(windows)]
pub fn measure_process(
    process_id: u32,
    warmup: Duration,
    duration: Duration,
) -> io::Result<ProcessMeasurement> {
    windows::measure_process(process_id, warmup, duration)
}

#[cfg(not(windows))]
pub fn measure_process(
    _process_id: u32,
    _warmup: Duration,
    _duration: Duration,
) -> io::Result<ProcessMeasurement> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "process measurement is currently implemented for Windows only",
    ))
}

#[cfg(windows)]
mod windows {
    use std::{
        io, mem, thread,
        time::{Duration, Instant},
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE},
        System::{
            ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        },
    };

    use super::ProcessMeasurement;

    const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
    const FILETIME_TICKS_PER_SECOND: f64 = 10_000_000.0;

    struct ProcessHandle(HANDLE);

    impl ProcessHandle {
        fn open(process_id: u32) -> io::Result<Self> {
            // SAFETY: OpenProcess receives a process ID returned by std::process::Child.
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(handle))
        }

        fn cpu_ticks(&self) -> io::Result<u64> {
            // SAFETY: all pointers refer to live FILETIME values for this synchronous call.
            let (creation, exit, kernel, user) = unsafe {
                let mut creation = mem::zeroed::<FILETIME>();
                let mut exit = mem::zeroed::<FILETIME>();
                let mut kernel = mem::zeroed::<FILETIME>();
                let mut user = mem::zeroed::<FILETIME>();
                if GetProcessTimes(self.0, &mut creation, &mut exit, &mut kernel, &mut user) == 0 {
                    return Err(io::Error::last_os_error());
                }
                (creation, exit, kernel, user)
            };
            let _ = (creation, exit);
            Ok(filetime_ticks(kernel).saturating_add(filetime_ticks(user)))
        }

        fn rss_bytes(&self) -> io::Result<u64> {
            // SAFETY: counters has the documented layout and remains live for the call.
            let counters = unsafe {
                let mut counters = mem::zeroed::<PROCESS_MEMORY_COUNTERS>();
                counters.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                if K32GetProcessMemoryInfo(
                    self.0,
                    &mut counters,
                    mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                ) == 0
                {
                    return Err(io::Error::last_os_error());
                }
                counters
            };
            Ok(counters.WorkingSetSize as u64)
        }
    }

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: this handle was returned by OpenProcess and is closed once here.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub fn measure_process(
        process_id: u32,
        warmup: Duration,
        duration: Duration,
    ) -> io::Result<ProcessMeasurement> {
        let process = ProcessHandle::open(process_id)?;
        thread::sleep(warmup);

        let started = Instant::now();
        let initial_ticks = process.cpu_ticks()?;
        let mut previous_ticks = initial_ticks;
        let mut previous_time = started;
        let mut cpu_samples = Vec::new();
        let mut maximum_rss_bytes = process.rss_bytes()?;

        while started.elapsed() < duration {
            thread::sleep(SAMPLE_INTERVAL.min(duration.saturating_sub(started.elapsed())));
            let now = Instant::now();
            let ticks = process.cpu_ticks()?;
            let wall_seconds = now.duration_since(previous_time).as_secs_f64();
            let cpu_seconds =
                ticks.saturating_sub(previous_ticks) as f64 / FILETIME_TICKS_PER_SECOND;
            cpu_samples.push(cpu_seconds / wall_seconds * 100.0);
            maximum_rss_bytes = maximum_rss_bytes.max(process.rss_bytes()?);
            previous_ticks = ticks;
            previous_time = now;
        }

        let elapsed_seconds = started.elapsed().as_secs_f64();
        let average_cpu_percent = previous_ticks.saturating_sub(initial_ticks) as f64
            / FILETIME_TICKS_PER_SECOND
            / elapsed_seconds
            * 100.0;
        let maximum_cpu_percent = cpu_samples.iter().copied().fold(0.0, f64::max);

        Ok(ProcessMeasurement {
            average_cpu_percent,
            maximum_cpu_percent,
            maximum_rss_bytes,
            samples: cpu_samples.len(),
        })
    }

    fn filetime_ticks(time: FILETIME) -> u64 {
        u64::from(time.dwLowDateTime) | (u64::from(time.dwHighDateTime) << 32)
    }

    #[cfg(test)]
    mod tests {
        use super::{FILETIME, filetime_ticks};

        #[test]
        fn combines_filetime_halves() {
            let time = FILETIME {
                dwLowDateTime: 0x89ab_cdef,
                dwHighDateTime: 0x0123_4567,
            };

            assert_eq!(filetime_ticks(time), 0x0123_4567_89ab_cdef);
        }
    }
}
