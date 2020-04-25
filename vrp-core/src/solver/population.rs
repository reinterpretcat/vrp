use crate::construction::heuristics::InsertionContext;
use crate::solver::{Individuum, Population};

/// An evolution aware implementation of `[Population]` trait.
pub struct DominancePopulation {}

impl DominancePopulation {
    /// Creates a new instance of `[EvoPopulation]`.
    pub fn new() -> Self {
        unimplemented!()
    }
}

impl Population for DominancePopulation {
    fn add(&mut self, _individuum: Individuum) {
        unimplemented!()
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        unimplemented!()
    }

    fn best(&self) -> Option<&Individuum> {
        unimplemented!()
    }

    fn select(&self) -> &(InsertionContext, usize) {
        unimplemented!()
    }

    fn size(&self) -> usize {
        unimplemented!()
    }
}
