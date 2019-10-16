use crate::construction::states::InsertionContext;
use crate::refinement::ruin::{create_insertion_context, RuinStrategy};
use crate::refinement::RefinementContext;

/// Removes random route from solution.
pub struct RandomRouteRemoval {
    /// Specifies minimum amount of removed routes.
    rmin: usize,
    /// Specifies maximum amount of removed routes.
    rmax: usize,
    /// Specifies threshold ratio of maximum removed routes.
    threshold: f64,
}

impl RandomRouteRemoval {
    pub fn new(rmin: usize, rmax: usize, threshold: f64) -> Self {
        Self { rmin, rmax, threshold }
    }
}

impl Default for RandomRouteRemoval {
    fn default() -> Self {
        Self::new(1, 3, 0.2)
    }
}

impl RuinStrategy for RandomRouteRemoval {
    fn ruin_solution(&self, refinement_ctx: &RefinementContext) -> Result<InsertionContext, String> {
        let individuum = refinement_ctx.individuum()?;
        let mut insertion_cxt = create_insertion_context(&refinement_ctx.problem, individuum, &refinement_ctx.random);

        // let max=

        unimplemented!()
    }
}
