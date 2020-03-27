/// Implements performance timer functionality, mostly exists due to problem
/// with `Instant` on wasm32 arch.
pub type Timer = actual::Timer;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
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
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {

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
    }

    fn now() -> f64 {
        web_sys::window().expect("no window in context").performance().expect("no performance available").now()
    }
}
