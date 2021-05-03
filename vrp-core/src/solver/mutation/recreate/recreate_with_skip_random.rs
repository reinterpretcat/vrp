use super::super::super::rand::prelude::SliceRandom;
use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::mutation::{ConfigurableRecreate, PhasedRecreate};
use crate::solver::population::SelectionPhase;
use crate::solver::RefinementContext;
use std::sync::Arc;

/// A recreate method which skips random jobs and routes.
pub struct RecreateWithSkipRandom {
    recreate: ConfigurableRecreate,
}

impl Default for RecreateWithSkipRandom {
    fn default() -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(SkipRandomJobSelector::default()),
                Box::new(SkipRandomRouteSelector::default()),
                Box::new(BestResultSelector::default()),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithSkipRandom {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}

impl RecreateWithSkipRandom {
    /// Creates `RecreateWithSkipRandom` as PhasedRecreate which runs only in exploration phase.
    pub fn default_explorative_phased(default_recreate: Arc<dyn Recreate + Send + Sync>) -> PhasedRecreate {
        let recreates = vec![
            (SelectionPhase::Initial, default_recreate.clone()),
            (SelectionPhase::Exploration, Arc::new(RecreateWithSkipRandom::default())),
            (SelectionPhase::Exploitation, default_recreate),
        ];

        PhasedRecreate { recreates: recreates.into_iter().collect() }
    }
}

struct SkipRandomJobSelector {}

impl Default for SkipRandomJobSelector {
    fn default() -> Self {
        Self {}
    }
}

impl JobSelector for SkipRandomJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        let skip = ctx.environment.random.uniform_int(2, 8) as usize;

        Box::new(ctx.solution.required.iter().skip(skip).cloned())
    }
}

struct SkipRandomRouteSelector {}

impl Default for SkipRandomRouteSelector {
    fn default() -> Self {
        Self {}
    }
}

impl RouteSelector for SkipRandomRouteSelector {
    fn select<'a>(
        &'a self,
        ctx: &'a mut InsertionContext,
        _jobs: &[Job],
    ) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        ctx.solution.routes.shuffle(&mut ctx.environment.random.get_rng());

        let skip = ctx.environment.random.uniform_int(0, 4);

        let skip = match (skip > ctx.solution.routes.len() as i32, ctx.solution.routes.len() > 1) {
            (true, true) => (skip - 1) as usize,
            (false, true) => 1,
            _ => 0,
        };

        Box::new(ctx.solution.routes.iter().skip(skip).cloned().chain(ctx.solution.registry.next()))
    }
}
