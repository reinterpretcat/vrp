use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::Random;
use std::sync::Arc;

/// A recreate method which takes a slice from jobs and routes.
pub struct RecreateWithSlice {
    recreate: ConfigurableRecreate,
}

impl RecreateWithSlice {
    /// Creates a new instance of `RecreateWithSlice`.
    pub fn new(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(SliceJobSelector::default()),
                Box::new(SliceRouteSelector::default()),
                Box::new(VariableLegSelector::new(random)),
                Box::new(BestResultSelector::default()),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithSlice {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}

#[derive(Default)]
struct SliceJobSelector {}

impl JobSelector for SliceJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        let required = ctx.solution.required.len() as i32;

        let take = ctx.environment.random.uniform_int(required / 4, required / 2) as usize;

        Box::new(ctx.solution.required.iter().take(take).cloned())
    }
}

#[derive(Default)]
struct SliceRouteSelector {}

impl RouteSelector for SliceRouteSelector {
    fn select<'a>(
        &'a self,
        ctx: &'a mut InsertionContext,
        _jobs: &[Job],
    ) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        ctx.solution.routes.shuffle(&mut ctx.environment.random.get_rng());

        let routes = ctx.solution.routes.len() as i32;

        let take = ctx.environment.random.uniform_int(routes / 4, routes / 2) as usize;

        Box::new(ctx.solution.routes.iter().take(take).cloned().chain(ctx.solution.registry.next()))
    }
}
