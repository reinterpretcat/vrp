use rosomaxa::prelude::Random;

/*struct FakeDistribution<T> {
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
}*/

pub fn create_fake_random(_ints: Vec<i32>, _reals: Vec<f64>) -> Random {
    todo!()
}

/*pub struct FakeRandom {
    ints: RwLock<FakeDistribution<i32>>,
    reals: RwLock<FakeDistribution<f64>>,
}

impl FakeRandom {
    pub fn new(ints: Vec<i32>, reals: Vec<f64>) -> Self {
        Self { ints: RwLock::new(FakeDistribution::new(ints)), reals: RwLock::new(FakeDistribution::new(reals)) }
    }
}

impl Random for FakeRandom {
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        assert!(min <= max);
        self.ints.write().unwrap().next()
    }

    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        assert!(min < max);
        self.reals.write().unwrap().next()
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
}*/
