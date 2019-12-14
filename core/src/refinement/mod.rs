extern crate rand;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::Problem;
use crate::utils::compare_floats;
use rand::prelude::*;
use std::cmp::Ordering;
use std::iter::once;
use std::sync::Arc;

/// Contains information needed to perform refinement.
pub struct RefinementContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Specifies solution population.
    pub population: Population,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

pub type Individuum = (InsertionContext, ObjectiveCost, usize);

pub struct Population {
    less_costs: Vec<Individuum>,
    less_unassigned: Vec<Individuum>,
    less_routes: Vec<Individuum>,

    batch_size: usize,
}

impl RefinementContext {
    pub fn new(problem: Arc<Problem>, batch_size: usize) -> Self {
        Self { problem, population: Population::new(batch_size), generation: 1 }
    }
}

impl Population {
    pub fn new(batch_size: usize) -> Self {
        Self { less_costs: vec![], less_routes: vec![], less_unassigned: vec![], batch_size }
    }

    /// Returns best solution by cost or minimum routes
    pub fn best(&self, minimum_routes: bool) -> Option<&Individuum> {
        if minimum_routes { self.less_routes() } else { self.less_costs() }.next()
    }

    /// Returns sorted collection discovered and accepted solutions
    /// with their cost and generations when they are discovered.
    pub fn less_costs<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(
            self.less_costs
                .iter()
                .zip(self.less_unassigned.iter())
                .zip(self.less_routes.iter())
                .flat_map(|((x, y), z)| once(x).chain(once(y)).chain(once(z))),
        )
    }

    /// Returns sorted collection by minimum routes amount.
    pub fn less_routes<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(
            self.less_routes
                .iter()
                .zip(self.less_unassigned.iter())
                .zip(self.less_costs.iter())
                .flat_map(|((x, y), z)| once(x).chain(once(y)).chain(once(z))),
        )
    }

    /// Returns total size of population.
    pub fn size(&self) -> usize {
        self.less_costs.len() + self.less_unassigned.len() + self.less_routes.len()
    }

    /// Adds solution to population
    pub fn add(&mut self, individuum: Individuum) {
        Self::add_to_queue(
            self.clone_individuum(&individuum),
            self.batch_size,
            &mut self.less_costs,
            |(_, a_cost, _), (_, b_cost, _)| compare_floats(a_cost.total(), b_cost.total()),
        );

        Self::add_to_queue(
            self.clone_individuum(&individuum),
            self.batch_size,
            &mut self.less_unassigned,
            |(a_ctx, a_cost, _), (b_ctx, b_cost, _)| match a_ctx
                .solution
                .unassigned
                .len()
                .cmp(&b_ctx.solution.unassigned.len())
            {
                Ordering::Equal => compare_floats(a_cost.total(), b_cost.total()),
                value @ _ => value,
            },
        );

        Self::add_to_queue(
            individuum,
            self.batch_size,
            &mut self.less_routes,
            |(a_ctx, a_cost, _), (b_ctx, b_cost, _)| match a_ctx.solution.routes.len().cmp(&b_ctx.solution.routes.len())
            {
                Ordering::Equal => compare_floats(a_cost.total(), b_cost.total()),
                value @ _ => value,
            },
        );
    }

    fn add_to_queue<F>(individuum: Individuum, batch_size: usize, individuums: &mut Vec<Individuum>, mut compare: F)
    where
        F: FnMut(&Individuum, &Individuum) -> Ordering,
    {
        individuums.push(individuum);
        individuums.sort_by(|a, b| compare(b, a));

        let best = individuums.pop().unwrap();
        individuums.shuffle(&mut rand::thread_rng());
        individuums.insert(0, best);
        individuums.truncate(batch_size);
    }

    fn clone_individuum(&self, individuum: &Individuum) -> Individuum {
        (individuum.0.deep_copy(), individuum.1.clone(), individuum.2)
    }
}

pub mod acceptance;
pub mod objectives;
pub mod recreate;
pub mod ruin;
pub mod selection;
pub mod termination;
