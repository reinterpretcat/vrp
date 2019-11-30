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
use std::sync::Arc;

/// Provides the way to run multiple ruin methods.
pub struct CompositeRuin {
    ruins: Vec<Vec<(Arc<dyn Ruin>, f64)>>,
    weights: Vec<usize>,
}

impl Default for CompositeRuin {
    fn default() -> Self {
        let adjusted_string = Arc::new(AdjustedStringRemoval::default());
        let worst_job = Arc::new(WorstJobRemoval::default());
        let random_job = Arc::new(RandomJobRemoval::default());
        let random_route = Arc::new(RandomRouteRemoval::default());
        Self::new(vec![
            (vec![(adjusted_string.clone(), 1.), (random_route.clone(), 0.05), (random_job.clone(), 0.05)], 100),
            (vec![(worst_job.clone(), 1.), (random_route.clone(), 0.05), (random_job.clone(), 0.05)], 100),
            (vec![(worst_job.clone(), 1.), (adjusted_string.clone(), 1.), (random_route.clone(), 0.05), (random_job.clone(), 0.05)], 20),
            (vec![(random_job.clone(), 1.), (random_route.clone(), 0.02)], 5),
            (vec![(random_route.clone(), 1.), (random_job.clone(), 0.02)], 5),
        ])
    }
}

impl CompositeRuin {
    pub fn new(ruins: Vec<(Vec<(Arc<dyn Ruin>, f64)>, usize)>) -> Self {
        let mut ruins = ruins;
        ruins.sort_by(|(_, a), (_, b)| b.cmp(&a));

        let weights = ruins.iter().map(|(_, weight)| *weight).collect();

        Self { ruins: ruins.into_iter().map(|(ruin, _)| ruin).collect(), weights }
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.random.clone();

        let index = insertion_ctx.random.weighted(self.weights.iter());

        let mut insertion_ctx = self
            .ruins
            .get(index)
            .unwrap()
            .iter()
            .filter(|(_, probability)| *probability > random.uniform_real(0., 1.))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}
