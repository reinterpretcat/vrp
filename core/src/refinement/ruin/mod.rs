use crate::construction::states::InsertionContext;

/// Specifies ruin strategy.
pub trait Ruin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod adjusted_string_removal;
pub use self::adjusted_string_removal::AdjustedStringRemoval;

mod random_route_removal;
pub use self::random_route_removal::RandomRouteRemoval;

mod random_job_removal;
pub use self::random_job_removal::RandomJobRemoval;
use crate::refinement::RefinementContext;

mod worst_jobs_removal;
pub use self::worst_jobs_removal::WorstJobRemoval;

/// Provides the way to run multiple ruin methods.
pub struct CompositeRuin {
    ruins: Vec<(Box<dyn Ruin>, f64)>,
}

impl Default for CompositeRuin {
    fn default() -> Self {
        Self {
            ruins: vec![
                (Box::new(AdjustedStringRemoval::default()), 0.5),
                (Box::new(WorstJobRemoval::default()), 0.5),
                (Box::new(RandomRouteRemoval::default()), 0.05),
                (Box::new(RandomJobRemoval::default()), 0.1),
            ],
        }
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.random.clone();

        let mut insertion_ctx = self
            .ruins
            .iter()
            .filter(|(_, probability)| *probability > random.uniform_real(0., 1.))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}
