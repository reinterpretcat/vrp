use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use rand::prelude::IteratorRandom;
use rosomaxa::prelude::{Float, Random};
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

custom_solution_state!(TabuList typeof TabuList);

/// A simple solution's tabu list to keep track of recently affected jobs and actors.
#[derive(Clone)]
pub struct TabuList {
    actors: HashSet<Arc<Actor>>,
    jobs: HashSet<Job>,
    max_actors: usize,
    max_jobs: usize,
    random: Arc<dyn Random>,
}

impl TabuList {
    /// Adds a job to the tabu list.
    pub fn add_job(&mut self, job: Job) {
        add_with_limits(job, &mut self.jobs, self.max_jobs, self.random.as_ref());
    }

    /// Adds an actor to the tabu list.
    pub fn add_actor(&mut self, actor: Arc<Actor>) {
        add_with_limits(actor, &mut self.actors, self.max_actors, self.random.as_ref());
    }

    /// Checks whether given an actor is in the tabu list.
    pub fn is_actor_tabu(&self, actor: &Actor) -> bool {
        self.actors.contains(actor)
    }

    /// Checks whether given a job is in the tabu list.
    pub fn is_job_tabu(&self, job: &Job) -> bool {
        self.jobs.contains(job)
    }

    /// Stores tabu list in insertion ctx.
    pub fn inject(self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.state.set_tabu_list(self);
    }
}

impl From<&InsertionContext> for TabuList {
    fn from(insertion_cxt: &InsertionContext) -> Self {
        let solution_ctx = &insertion_cxt.solution;
        let routes = solution_ctx.routes.len();
        let jobs = solution_ctx.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum::<usize>();

        let max_actors = match routes {
            _ if routes <= 1 => 0,
            _ => (routes as Float * 0.5).trunc() as usize,
        };

        let max_jobs = match jobs {
            _ if jobs <= 1 => 0,
            _ => (jobs as Float * 0.5).trunc() as usize,
        };

        let other_tabu_list = solution_ctx.state.get_tabu_list().cloned().unwrap_or_else(|| TabuList {
            actors: Default::default(),
            jobs: Default::default(),
            max_actors,
            max_jobs,
            random: insertion_cxt.environment.random.clone(),
        });

        TabuList { max_actors, max_jobs, ..other_tabu_list }
    }
}

fn add_with_limits<T: Clone + Eq + PartialEq + Hash>(
    new_item: T,
    old_items: &mut HashSet<T>,
    limits: usize,
    random: &dyn Random,
) {
    // NOTE do not use tabu list when limit is zero
    if limits == 0 {
        return;
    }

    if old_items.len() == limits
        && let Some(item) = old_items.iter().choose(&mut random.get_rng()).cloned()
    {
        old_items.remove(&item);
    }

    old_items.insert(new_item);
}
