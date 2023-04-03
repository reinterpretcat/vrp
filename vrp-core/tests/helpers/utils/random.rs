use rosomaxa::prelude::{Random, RandomGen};

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

    #[allow(clippy::mut_from_ref)]
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

    fn is_head_not_tails(&self) -> bool {
        self.uniform_int(1, 2) == 1
    }

    fn is_hit(&self, probability: f64) -> bool {
        self.uniform_real(0., 1.) < probability
    }

    fn weighted(&self, _: &[usize]) -> usize {
        todo!()
    }

    fn get_rng(&self) -> RandomGen {
        RandomGen::new_repeatable()
    }
}
