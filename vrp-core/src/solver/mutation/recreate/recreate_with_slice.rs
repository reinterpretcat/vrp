use super::super::super::rand::prelude::SliceRandom;
use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::mutation::ConfigurableRecreate;
use crate::solver::RefinementContext;

/// A recreate method which takes a slice from jobs and routes.
pub struct RecreateWithSlice {
    recreate: ConfigurableRecreate,
}

impl Default for RecreateWithSlice {
    fn default() -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(SliceJobSelector::default()),
                Box::new(SliceRouteSelector::default()),
                Box::new(AllLegSelector::default()),
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

struct SliceJobSelector {}

impl Default for SliceJobSelector {
    fn default() -> Self {
        Self {}
    }
}

impl JobSelector for SliceJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        let required = ctx.solution.required.len() as i32;

        let take = ctx.environment.random.uniform_int(required / 4, required / 2) as usize;

        Box::new(ctx.solution.required.iter().take(take).cloned())
    }
}

struct SliceRouteSelector {}

impl Default for SliceRouteSelector {
    fn default() -> Self {
        Self {}
    }
}

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
