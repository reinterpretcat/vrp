use crate::prelude::*;
use rand::SeedableRng;

struct FakeDistribution<T> {
    values: Vec<T>,
}

impl<T> FakeDistribution<T> {
    pub fn new(values: Vec<T>) -> Self {
        let mut values = values;
        values.reverse();
        Self { values }
    }

    pub fn next(&mut self) -> T {
        self.values.pop().unwrap()
    }
}

pub struct FakeRandom {
    ints: FakeDistribution<i32>,
    reals: FakeDistribution<f64>,
}

impl FakeRandom {
    pub fn new(ints: Vec<i32>, reals: Vec<f64>) -> Self {
        Self { ints: FakeDistribution::new(ints), reals: FakeDistribution::new(reals) }
    }

    unsafe fn const_cast(&self) -> &mut Self {
        let const_ptr = self as *const Self;
        let mut_ptr = const_ptr as *mut Self;
        &mut *mut_ptr
    }
}

impl Random for FakeRandom {
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        assert!(min <= max);
        unsafe { self.const_cast().ints.next() }
    }

    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        assert!(min < max);
        unsafe { self.const_cast().reals.next() }
    }

    fn get_rng(&self) -> RandomGen {
        RandomGen::seed_from_u64(0)
    }
}

pub struct EchoRandom {
    use_min: bool,
}

impl EchoRandom {
    pub fn new(use_min: bool) -> Self {
        Self { use_min }
    }
}

impl Random for EchoRandom {
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        if self.use_min {
            min
        } else {
            max
        }
    }

    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        if self.use_min {
            min
        } else {
            max
        }
    }

    fn get_rng(&self) -> RandomGen {
        RandomGen::seed_from_u64(0)
    }
}
