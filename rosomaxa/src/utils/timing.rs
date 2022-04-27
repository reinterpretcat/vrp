use std::time::Duration;

/// Implements performance timer functionality, mostly exists due to problem
/// with `Instant` on wasm32 arch.
pub type Timer = actual::Timer;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    use super::*;
    use std::time::Instant;

    #[derive(Clone)]
    pub struct Timer {
        start: Instant,
    }

    impl Timer {
        pub fn start() -> Self {
            Self { start: Instant::now() }
        }

        pub fn elapsed_secs(&self) -> u64 {
            (Instant::now() - self.start).as_secs()
        }

        pub fn elapsed_secs_as_f64(&self) -> f64 {
            (Instant::now() - self.start).as_secs_f64()
        }

        pub fn elapsed_millis(&self) -> u128 {
            (Instant::now() - self.start).as_millis()
        }

        pub fn measure_duration<R, F: Fn() -> R>(action: F) -> (R, Duration) {
            measure_duration(action)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    use super::*;

    #[derive(Clone)]
    pub struct Timer {
        start: f64,
    }

    impl Timer {
        pub fn start() -> Self {
            Self { start: now() }
        }

        pub fn elapsed_secs(&self) -> u64 {
            self.elapsed_secs_as_f64().round() as u64
        }

        pub fn elapsed_secs_as_f64(&self) -> f64 {
            (now() - self.start) / 1000.
        }

        pub fn elapsed_millis(&self) -> u128 {
            (now() - self.start) as u128
        }

        pub fn measure_duration<R, F: Fn() -> R>(action: F) -> (R, Duration) {
            measure_duration(action)
        }
    }

    fn now() -> f64 {
        js_sys::Date::new_0().get_time() as f64
    }
}

fn measure_duration<R, F: Fn() -> R>(action: F) -> (R, Duration) {
    let start = Timer::start();
    let result = action();
    let elapsed = start.elapsed_millis();

    (result, Duration::from_millis(elapsed as u64))
}
