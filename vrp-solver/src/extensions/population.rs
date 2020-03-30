use std::collections::VecDeque;
use vrp_core::refinement::{Individuum, Population};

pub struct SimplePopulation {
    individuums: VecDeque<Individuum>,
    size: usize,
}

impl Population for SimplePopulation {
    fn add(&mut self, individuum: Individuum) {
        self.individuums.push_front(individuum);
        self.individuums.truncate(self.size);
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(self.individuums.iter())
    }

    fn best(&self) -> Option<&Individuum> {
        self.individuums.front()
    }

    fn size(&self) -> usize {
        self.individuums.len()
    }
}

impl SimplePopulation {
    pub fn new(size: usize) -> Self {
        assert!(size > 1);

        Self { individuums: VecDeque::default(), size }
    }
}
