use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rosomaxa::prelude::{DefaultRandom, Random};

/// A recreate method which takes a slice from jobs and routes.
pub struct RecreateWithSlice {
    recreate: ConfigurableRecreate,
}

impl RecreateWithSlice {
    /// Creates a new instance of `RecreateWithSlice`.
    pub fn new(random: DefaultRandom) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<SliceJobSelector>::default(),
                Box::<SliceRouteSelector>::default(),
                LegSelection::Stochastic(random.clone()),
                ResultSelection::Stochastic(ResultSelectorProvider::new_default(random)),
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
    fn select<'a>(&'a self, insertion_ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        let required = insertion_ctx.solution.required.len() as i32;

        let take = insertion_ctx.environment.random.uniform_int(required / 4, required / 2) as usize;

        Box::new(insertion_ctx.solution.required.iter().take(take))
    }
}

#[derive(Default)]
struct SliceRouteSelector {}

impl RouteSelector for SliceRouteSelector {
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a InsertionContext,
        _jobs: &[&Job],
    ) -> Box<dyn Iterator<Item = &'a RouteContext> + 'a> {
        let routes = insertion_ctx.solution.routes.len() as i32;

        let take = insertion_ctx.environment.random.uniform_int(routes / 4, routes / 2) as usize;

        Box::new(insertion_ctx.solution.routes.iter().take(take).chain(insertion_ctx.solution.registry.next_route()))
    }
}
