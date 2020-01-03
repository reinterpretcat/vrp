use std::cmp::Ordering;
use vrp_core::refinement::{Individuum, Population};
use vrp_core::utils::compare_floats;

pub struct DiversePopulation {
    less_costs: Vec<Individuum>,
    less_unassigned: Vec<Individuum>,
    less_routes: Vec<Individuum>,

    minimize_routes: bool,
    batch_size: usize,
}

impl Population for DiversePopulation {
    fn add(&mut self, individuum: Individuum) {
        Self::add_to_queue(
            self.clone_individuum(&individuum),
            if self.minimize_routes { 2 } else { self.batch_size },
            &mut self.less_costs,
            |(_, a_cost, _), (_, b_cost, _)| compare_floats(a_cost.total(), b_cost.total()),
        );

        Self::add_to_queue(
            self.clone_individuum(&individuum),
            1,
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
            if self.minimize_routes { self.batch_size } else { 2 },
            &mut self.less_routes,
            |(a_ctx, a_cost, _), (b_ctx, b_cost, _)| match a_ctx.solution.routes.len().cmp(&b_ctx.solution.routes.len())
            {
                Ordering::Equal => compare_floats(a_cost.total(), b_cost.total()),
                value @ _ => value,
            },
        );
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        if self.minimize_routes {
            self.less_routes()
        } else {
            self.less_costs()
        }
    }

    fn best(&self) -> Option<&Individuum> {
        self.all().next()
    }

    fn size(&self) -> usize {
        self.less_costs.len() + self.less_unassigned.len() + self.less_routes.len()
    }
}

impl DiversePopulation {
    pub fn new(minimize_routes: bool, batch_size: usize) -> Self {
        assert!(batch_size > 1);
        Self { less_costs: vec![], less_routes: vec![], less_unassigned: vec![], minimize_routes, batch_size }
    }

    /// Returns sorted collection discovered and accepted solutions
    /// with their cost and generations when they are discovered.
    fn less_costs<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(self.less_costs.iter().chain(self.less_unassigned.iter()).chain(self.less_routes.iter()))
    }

    /// Returns sorted collection by minimum routes amount.
    fn less_routes<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(self.less_routes.iter().chain(self.less_unassigned.iter()).chain(self.less_costs.iter()))
    }

    fn add_to_queue<F>(individuum: Individuum, batch_size: usize, individuums: &mut Vec<Individuum>, mut compare: F)
    where
        F: FnMut(&Individuum, &Individuum) -> Ordering,
    {
        individuums.truncate(batch_size - 1);

        individuums.push(individuum);
        individuums.sort_by(|a, b| compare(a, b));
    }

    fn clone_individuum(&self, individuum: &Individuum) -> Individuum {
        (individuum.0.deep_copy(), individuum.1.clone(), individuum.2)
    }
}
