use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use crate::solver::TABU_LIST_KEY;
use hashbrown::HashSet;
use rand::prelude::IteratorRandom;
use rosomaxa::prelude::Random;
use std::hash::Hash;
use std::sync::Arc;

/// A simple solution's tabu list to keep track of recently affected jobs and actors.
#[derive(Clone)]
pub struct TabuList {
    actors: HashSet<Arc<Actor>>,
    jobs: HashSet<Job>,
    max_actors: usize,
    max_jobs: usize,
    random: Arc<dyn Random + Send + Sync>,
}

impl TabuList {
    /// Adds job to tabu list.
    pub fn add_job(&mut self, job: Job) {
        add_with_limits(job, &mut self.jobs, self.max_jobs, self.random.as_ref());
    }

    /// Adds actor to tabu list.
    pub fn add_actor(&mut self, actor: Arc<Actor>) {
        add_with_limits(actor, &mut self.actors, self.max_actors, self.random.as_ref());
    }

    /// Checks whether given actor is in tabu list.
    pub fn is_actor_tabu(&self, actor: &Actor) -> bool {
        self.actors.contains(actor)
    }

    /// Checks whether given job is in tabu list.
    pub fn is_job_tabu(&self, job: &Job) -> bool {
        self.jobs.contains(job)
    }

    /// Stores tabu list in insertion ctx.
    pub fn inject(self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.state.insert(TABU_LIST_KEY, Arc::new(self));
    }
}

impl From<&InsertionContext> for TabuList {
    fn from(insertion_cxt: &InsertionContext) -> Self {
        let solution_ctx = &insertion_cxt.solution;
        let routes = solution_ctx.routes.len();
        let jobs = solution_ctx.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum::<usize>();

        let max_actors = match routes {
            _ if routes <= 1 => 0,
            _ => (routes as f64 * 0.5).trunc() as usize,
        };

        let max_jobs = match jobs {
            _ if jobs <= 1 => 0,
            _ => (jobs as f64 * 0.5).trunc() as usize,
        };

        TabuList {
            max_actors,
            max_jobs,
            ..solution_ctx.state.get(&TABU_LIST_KEY).and_then(|s| s.downcast_ref::<TabuList>().cloned()).unwrap_or_else(
                || TabuList {
                    actors: Default::default(),
                    jobs: Default::default(),
                    max_actors,
                    max_jobs,
                    random: insertion_cxt.environment.random.clone(),
                },
            )
        }
    }
}

fn add_with_limits<T: Clone + Eq + PartialEq + Hash>(
    new_item: T,
    old_items: &mut HashSet<T>,
    limits: usize,
    random: &(dyn Random + Send + Sync),
) {
    // NOTE do not use tabu list when limit is zero
    if limits == 0 {
        return;
    }

    if old_items.len() == limits {
        if let Some(item) = old_items.iter().choose(&mut random.get_rng()).cloned() {
            old_items.remove(&item);
        }
    }

    old_items.insert(new_item);
}
