use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::{ConfigurableRecreate, PhasedRecreate};
use rosomaxa::prelude::*;
use std::sync::Arc;

/// A recreate method which skips random jobs and routes.
pub struct RecreateWithSkipRandom {
    recreate: ConfigurableRecreate,
}

impl RecreateWithSkipRandom {
    /// Creates a new instance of `RecreateWithSkipRandom`.
    pub fn new(random: Arc<dyn Random>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<SkipRandomJobSelector>::default(),
                Box::<SkipRandomRouteSelector>::default(),
                LegSelection::Stochastic(random.clone()),
                ResultSelection::Stochastic(ResultSelectorProvider::new_default(random)),
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
    /// Creates `RecreateWithSkipRandom` as `PhasedRecreate` which runs only in exploration phase.
    pub fn default_explorative_phased(default_recreate: Arc<dyn Recreate>, random: Arc<dyn Random>) -> PhasedRecreate {
        let recreates = vec![
            (SelectionPhase::Initial, default_recreate.clone()),
            (SelectionPhase::Exploration, Arc::new(RecreateWithSkipRandom::new(random))),
            (SelectionPhase::Exploitation, default_recreate),
        ];

        PhasedRecreate { recreates: recreates.into_iter().collect() }
    }
}

#[derive(Default)]
struct SkipRandomJobSelector {}

impl JobSelector for SkipRandomJobSelector {
    fn select<'a>(&'a self, insertion_ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        let skip = insertion_ctx.environment.random.uniform_int(2, 8) as usize;

        Box::new(insertion_ctx.solution.required.iter().skip(skip))
    }
}

#[derive(Default)]
struct SkipRandomRouteSelector {}

impl RouteSelector for SkipRandomRouteSelector {
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a InsertionContext,
        _: &[&Job],
    ) -> Box<dyn Iterator<Item = &'a RouteContext> + 'a> {
        let skip = insertion_ctx.environment.random.uniform_int(0, 4);

        let skip = match (skip > insertion_ctx.solution.routes.len() as i32, insertion_ctx.solution.routes.len() > 1) {
            (true, true) => (skip - 1) as usize,
            (false, true) => 1,
            _ => 0,
        };

        Box::new(insertion_ctx.solution.routes.iter().skip(skip).chain(insertion_ctx.solution.registry.next_route()))
    }
}
