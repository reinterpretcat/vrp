use crate::utils::Float;
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

        pub fn elapsed_secs_as_float(&self) -> Float {
            (Instant::now() - self.start).as_secs_f64() as Float
        }

        pub fn elapsed_millis(&self) -> u128 {
            (Instant::now() - self.start).as_millis()
        }

        pub fn measure_duration<R, F: FnOnce() -> R>(action_fn: F) -> (R, Duration) {
            measure_duration(action_fn)
        }

        pub fn measure_duration_with_callback<R, FA, FM>(action_fn: FA, callback_fn: FM) -> R
        where
            FA: FnOnce() -> R,
            FM: FnOnce(Duration),
        {
            let (result, duration) = measure_duration(action_fn);
            callback_fn(duration);

            result
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    use super::*;

    #[derive(Clone)]
    pub struct Timer {
        start: Float,
    }

    impl Timer {
        pub fn start() -> Self {
            Self { start: now() }
        }

        pub fn elapsed_secs(&self) -> u64 {
            self.elapsed_secs_as_float().round() as u64
        }

        pub fn elapsed_secs_as_float(&self) -> Float {
            (now() - self.start) / 1000.
        }

        pub fn elapsed_millis(&self) -> u128 {
            (now() - self.start) as u128
        }

        pub fn measure_duration<R, F: FnOnce() -> R>(action_fn: F) -> (R, Duration) {
            measure_duration(action_fn)
        }

        pub fn measure_duration_with_callback<R, FA, FM>(action_fn: FA, callback_fn: FM) -> R
        where
            FA: FnOnce() -> R,
            FM: FnOnce(Duration),
        {
            let (result, duration) = measure_duration(action_fn);
            callback_fn(duration);

            result
        }
    }

    fn now() -> Float {
        js_sys::Date::new_0().get_time() as Float
    }
}

fn measure_duration<R, F: FnOnce() -> R>(action_fn: F) -> (R, Duration) {
    let start = Timer::start();
    let result = action_fn();
    let elapsed = start.elapsed_millis();

    (result, Duration::from_millis(elapsed as u64))
}
