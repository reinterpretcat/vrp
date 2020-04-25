use crate::construction::heuristics::InsertionContext;
use crate::solver::{Individual, Population};

/// An evolution aware implementation of `[Population]` trait.
pub struct DominancePopulation {}

impl DominancePopulation {
    /// Creates a new instance of `[EvoPopulation]`.
    pub fn new() -> Self {
        unimplemented!()
    }
}

impl Population for DominancePopulation {
    fn add(&mut self, _individual: Individual) {
        unimplemented!()
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        unimplemented!()
    }

    fn best(&self) -> Option<&Individual> {
        unimplemented!()
    }

    fn select(&self) -> &(InsertionContext, usize) {
        unimplemented!()
    }

    fn size(&self) -> usize {
        unimplemented!()
    }
}
