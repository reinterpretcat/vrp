use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;

pub trait Termination {
    fn is_termination(
        &mut self,
        refinement_ctx: &RefinementContext,
        solution: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool;
}

mod max_generation;
pub use self::max_generation::MaxGeneration;

mod max_time;
pub use self::max_time::MaxTime;

mod variation_coefficient;
pub use self::variation_coefficient::VariationCoefficient;

pub struct CompositeTermination {
    terminations: Vec<Box<dyn Termination>>,
}

impl CompositeTermination {
    pub fn new(terminations: Vec<Box<dyn Termination>>) -> Self {
        Self { terminations }
    }
}

impl Default for CompositeTermination {
    fn default() -> Self {
        Self::new(vec![Box::new(MaxGeneration::default()), Box::new(VariationCoefficient::default())])
    }
}

impl Termination for CompositeTermination {
    fn is_termination(
        &mut self,
        refinement_ctx: &RefinementContext,
        solution: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool {
        self.terminations
            .iter_mut()
            .any(|t| t.is_termination(refinement_ctx, (solution.0, solution.1.clone(), solution.2)))
    }
}
