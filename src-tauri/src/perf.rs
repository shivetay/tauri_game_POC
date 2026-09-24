//! Frame and process performance counters for the debug overlay.

use std::time::Instant;

/// Smoothed FPS + process CPU% (Windows). Updated once per egui frame.
pub struct PerfStats {
    fps: f32,
    /// Process CPU across all cores, 0–100 * N_cpus normalized to 0–100.
    cpu_percent: f32,
    last_frame: Instant,
    last_cpu_wall: Instant,
    last_process_100ns: Option<u64>,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfStats {
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            cpu_percent: 0.0,
            last_frame: Instant::now(),
            last_cpu_wall: Instant::now(),
            last_process_100ns: process_cpu_100ns(),
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn cpu_percent(&self) -> f32 {
        self.cpu_percent
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        if dt > 1e-6 && dt < 1.0 {
            let instant_fps = (1.0 / dt) as f32;
            self.fps = if self.fps <= 0.0 {
                instant_fps
            } else {
                self.fps * 0.9 + instant_fps * 0.1
            };
        }

        let wall = now.duration_since(self.last_cpu_wall).as_secs_f64();
        if wall < 0.25 {
            return;
        }
        let Some(prev) = self.last_process_100ns else {
            self.last_process_100ns = process_cpu_100ns();
            self.last_cpu_wall = now;
            return;
        };
        let Some(curr) = process_cpu_100ns() else {
            return;
        };
        self.last_process_100ns = Some(curr);
        self.last_cpu_wall = now;

        let process_secs = (curr.saturating_sub(prev)) as f64 / 10_000_000.0;
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1) as f64;
        let pct = (process_secs / wall / cores) * 100.0;
        self.cpu_percent = pct.clamp(0.0, 100.0) as f32;
    }
}

#[cfg(windows)]
fn process_cpu_100ns() -> Option<u64> {
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        fn as_u64(self) -> u64 {
            ((self.high as u64) << 32) | self.low as u64
        }
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn GetProcessTimes(
            process: *mut core::ffi::c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }

    unsafe {
        let mut creation = FileTime { low: 0, high: 0 };
        let mut exit = FileTime { low: 0, high: 0 };
        let mut kernel = FileTime { low: 0, high: 0 };
        let mut user = FileTime { low: 0, high: 0 };
        let ok = GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        );
        if ok == 0 {
            return None;
        }
        Some(kernel.as_u64() + user.as_u64())
    }
}

#[cfg(not(windows))]
fn process_cpu_100ns() -> Option<u64> {
    None
}
