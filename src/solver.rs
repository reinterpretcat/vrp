use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::recreate::{Recreate, RecreateWithCheapest};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::termination::{MaxGeneration, Termination};

pub struct Solver {
    recreate: Box<dyn Recreate>,
    ruin: Box<dyn Ruin>,
    acceptance: Box<dyn Acceptance>,
    termination: Box<dyn Termination>,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(RecreateWithCheapest::default()),
            Box::new(CompositeRuin::default()),
            Box::new(Greedy::default()),
            Box::new(MaxGeneration::default()),
        )
    }
}

impl Solver {
    pub fn new(
        recreate: Box<dyn Recreate>,
        ruin: Box<dyn Ruin>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
    ) -> Self {
        Self { recreate, ruin, acceptance, termination }
    }
}
