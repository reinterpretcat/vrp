#[cfg(test)]
#[path = "../../../tests/unit/solver/search/lkh_search_test.rs"]
mod lkh_search_test;

use super::*;
use crate::{
    algorithms::lkh::*,
    construction::probing::repair_solution_from_unknown,
    models::solution::Tour,
    prelude::{Cost, Location, RouteContext, TransportCost},
};
use rosomaxa::utils::{ParallelismScope, parallel_foreach_mut};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

/// A search mode for LKH algorithm.
#[derive(Clone, Copy, Debug)]
pub enum LKHSearchMode {
    /// Accepts only improvements.
    ImprovementOnly,
    /// Accepts all solutions.
    Diverse,
}

/// A search operator which uses modified LKH algorithm to optimize routes.
#[derive(Clone, Debug)]
pub struct LKHSearch {
    mode: LKHSearchMode,
}

impl HeuristicSearchOperator for LKHSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let transport = solution.problem.transport.as_ref();

        let mut new_solution = solution.deep_copy();

        // apply LKH optimization to each route separately
        parallel_foreach_mut(&mut new_solution.solution.routes, ParallelismScope::Local, |route_ctx| {
            optimize_route(route_ctx, transport)
        });

        self.repair_routes(new_solution, solution)
    }
}

impl LKHSearch {
    /// Creates a new instance of [LKHSearch].
    pub fn new(mode: LKHSearchMode) -> Self {
        Self { mode }
    }

    fn repair_routes(&self, new_solution: InsertionContext, orig_solution: &InsertionContext) -> InsertionContext {
        // repair entire solution
        // TODO can be optimized to avoid full reconstruction and repair only routes that were changed
        let mut new_solution = repair_solution_from_unknown(&new_solution, &|| {
            InsertionContext::new(orig_solution.problem.clone(), orig_solution.environment.clone())
        });

        // accept the fact that solution can be worse
        if matches!(self.mode, LKHSearchMode::Diverse) {
            return new_solution;
        }

        // get all routes from original solution indexed by actor
        let orig_routes: HashMap<_, _> =
            orig_solution.solution.routes.iter().map(|route| (&route.route().actor, route)).collect();

        // get set of actors already present in new solution
        let existing_actors: HashSet<_> =
            new_solution.solution.routes.iter().map(|route| route.route().actor.clone()).collect();

        // add any missing routes from original solution
        orig_routes.iter().filter(|&(&actor, _)| !existing_actors.contains(actor)).for_each(|(_, &orig_route_ctx)| {
            let route_ctx = orig_route_ctx.deep_copy();
            new_solution.solution.registry.use_route(&route_ctx);
            new_solution.solution.routes.push(route_ctx);
        });

        // ensure routes have at least as many jobs as in original solution
        new_solution
            .solution
            .routes
            .iter_mut()
            .filter_map(|route_ctx| {
                orig_routes.get(&route_ctx.route().actor).map(|orig_route_ctx| (route_ctx, orig_route_ctx))
            })
            .filter(|(route_ctx, orig_route_ctx)| {
                orig_route_ctx.route().tour.job_count() > route_ctx.route().tour.job_count()
            })
            .for_each(|(route_ctx, orig_route_ctx)| {
                *route_ctx = orig_route_ctx.deep_copy();
            });

        // restore original unassigned jobs
        new_solution.solution.unassigned = orig_solution.solution.unassigned.clone();

        // recalculate solution state if we do
        new_solution.restore();

        new_solution
    }
}

fn optimize_route(route_ctx: &mut RouteContext, transport: &dyn TransportCost) {
    // skip routes that are too small for optimization
    if route_ctx.route().tour.total() <= 3 {
        return;
    }

    let path = route_to_path(route_ctx);

    // build the adjacency matrix for LKH
    let adjacency = CostMatrix::new(route_ctx, transport);

    // run LKH algorithm and take last optimized path if it is different from original
    let optimized = match lkh_optimize(adjacency, path.clone()).last().filter(|optimized| **optimized != path) {
        Some(opt) => opt.clone(),
        None => return,
    };

    rearrange_route(route_ctx, optimized);
}

/// Converts a [RouteContext] to a [Path] as vector of sequential indices.
fn route_to_path(route_ctx: &RouteContext) -> Path {
    debug_assert!(route_ctx.route().tour.total() > 3);

    get_activity_range(&route_ctx.route().tour).collect()
}

/// Reshufles [RouteContext] according to [Path] ordering.
fn rearrange_route(route_ctx: &mut RouteContext, mut path: Path) {
    let range = get_activity_range(&route_ctx.route().tour);
    let activities = route_ctx.route_mut().tour.activities_mut();

    // rearrange activities using swaps
    for i in range.rev() {
        let current_idx = path[i];

        // skip if activity is already in the right position
        if current_idx == i {
            continue;
        }

        // swap the activity to its target position
        activities.swap(current_idx, i);
        if let Some(i_pos) = path.iter().position(|&p| p == i) {
            path.swap(i, i_pos);
        }
    }
}

/// Gets a range of activity indices for usage.
fn get_activity_range(tour: &Tour) -> Range<usize> {
    debug_assert!(tour.total() > 1);

    // offset is used to skip the last activity if it has the same location as the first one
    // current existing LKH implementation assumes that last point is the same as first, so we need to skip it.
    // TODO: if end point is not the same, then we do not skip it, but LKH will will consider returning to the start point.
    let has_same_endpoints =
        tour.start().zip(tour.end()).filter(|(start, end)| start.place.location == end.place.location).is_some();

    0..tour.total() - if has_same_endpoints { 1 } else { 0 }
}

/// Provides an implementation of [AdjacencySpec] for LKH algorithm.
struct CostMatrix {
    costs: Vec<Cost>,
    size: usize,
    neighbourhood: Vec<Node>,
}

impl CostMatrix {
    fn new(route_ctx: &RouteContext, transport: &dyn TransportCost) -> Self {
        let profile = route_ctx.route().actor.vehicle.profile.clone();
        let tour = &route_ctx.route().tour;

        // extract locations from activities
        let locations: Vec<Location> =
            get_activity_range(tour).filter_map(|idx| tour.get(idx)).map(|a| a.place.location).collect();
        let size = locations.len();
        // Approximate distance is time-independent and LKH queries the same route edges repeatedly.
        let costs = (0..size)
            .flat_map(|from| {
                let profile = &profile;
                let locations = &locations;
                (0..size).map(move |to| {
                    if from == to {
                        Cost::default()
                    } else {
                        transport.distance_approx(profile, locations[from], locations[to])
                    }
                })
            })
            .collect::<Vec<_>>();

        // Store all rows in one allocation. Each row contains every other node sorted by distance.
        let row_size = size.saturating_sub(1);
        let mut neighbourhood = Vec::with_capacity(size * row_size);
        for from in 0..size {
            let offset = neighbourhood.len();
            neighbourhood.extend((0..size).filter(|&to| from != to));
            neighbourhood[offset..]
                .sort_by(|&left, &right| costs[from * size + left].total_cmp(&costs[from * size + right]));
        }

        CostMatrix { costs, size, neighbourhood }
    }
}

impl AdjacencySpec for CostMatrix {
    fn cost(&self, edge: &Edge) -> Cost {
        let &(from, to) = edge;
        // NOTE: LKH assumes symmetric distances
        // TODO: handle one-direction reachable only locations
        let (from, to) = if from > to { (to, from) } else { (from, to) };

        self.costs[from * self.size + to]
    }

    fn neighbours(&self, node: Node) -> &[Node] {
        let row_size = self.size.saturating_sub(1);
        let offset = node * row_size;

        &self.neighbourhood[offset..offset + row_size]
    }
}
