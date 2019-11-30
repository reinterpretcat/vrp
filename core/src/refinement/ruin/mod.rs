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
        let adjusted_string_default = Arc::new(AdjustedStringRemoval::default());
        let adjusted_string_agressive = Arc::new(AdjustedStringRemoval::new(30, 120, 0.02));

        let worst_job_default = Arc::new(WorstJobRemoval::default());
        let worst_job_agressive = Arc::new(WorstJobRemoval::new(120, 8, (4, 16)));

        let random_job_default = Arc::new(RandomJobRemoval::default());
        let random_job_agressive = Arc::new(RandomJobRemoval::new(30, 120, 0.2));

        let random_route_default = Arc::new(RandomRouteRemoval::default());
        let random_route_agressive = Arc::new(RandomRouteRemoval::new(5, 20, 0.2));

        Self::new(vec![
            (
                vec![
                    (adjusted_string_default.clone(), 1.),
                    (random_route_default.clone(), 0.05),
                    (random_job_default.clone(), 0.05),
                ],
                100,
            ),
            (vec![(adjusted_string_agressive.clone(), 1.)], 3),
            (
                vec![
                    (worst_job_default.clone(), 1.),
                    (random_route_default.clone(), 0.05),
                    (random_job_default.clone(), 0.05),
                ],
                100,
            ),
            (vec![(worst_job_agressive.clone(), 1.)], 3),
            (vec![(worst_job_default.clone(), 1.), (adjusted_string_default.clone(), 1.)], 20),
            (vec![(random_job_default.clone(), 1.), (random_route_default.clone(), 0.02)], 2),
            (vec![(random_job_agressive.clone(), 1.)], 1),
            (vec![(random_route_default.clone(), 1.), (random_job_default.clone(), 0.02)], 2),
            (vec![(random_route_agressive.clone(), 1.)], 1),
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
