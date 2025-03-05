use super::*;
use crate::{
    algorithms::lkh::*,
    construction::{heuristics::finalize_insertion_ctx, probing::repair_solution_from_unknown},
    models::common::Profile,
    prelude::{Cost, Location, RouteContext, TransportCost},
};
use rosomaxa::utils::{parallel_foreach_mut, parallel_into_collect};

/// A search mode for LKH algorithm.
#[derive(Clone, Copy, Debug)]
pub enum LKHSearchMode {
    /// Accepts only improvements.
    ImprovementOnly,
    /// Accepts all solutions.
    Diverse,
}

/// A search operator which uses modified LKH algorithm to optimize routes.
#[derive(Debug)]
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
        parallel_foreach_mut(&mut new_solution.solution.routes, |route_ctx| optimize_route(route_ctx, transport));

        self.repair_routes(new_solution, solution)
    }
}

impl LKHSearch {
    /// Creates a new instance of [LKHSearch].
    pub fn new(mode: LKHSearchMode) -> Self {
        Self { mode }
    }

    fn repair_routes(&self, mut new_solution: InsertionContext, orig_solution: &InsertionContext) -> InsertionContext {
        // repair entire solution
        // TODO can be optimized to avoid full reconstruction and repair only routes that were changed
        let mut new_solution = repair_solution_from_unknown(&new_solution, &|| {
            InsertionContext::new(orig_solution.problem.clone(), orig_solution.environment.clone())
        });

        // accept the fact that solution can be worse
        if matches!(self.mode, LKHSearchMode::Diverse) {
            return new_solution;
        }

        // keep original route if it is better
        new_solution.solution.routes.iter_mut().zip(orig_solution.solution.routes.iter()).for_each(
            |(new_route_ctx, orig_route_ctx)| {
                debug_assert!(new_route_ctx.route().actor == orig_route_ctx.route().actor);

                if orig_route_ctx.route().tour.job_count() > new_route_ctx.route().tour.job_count() {
                    *new_route_ctx = orig_route_ctx.deep_copy();
                }
            },
        );

        // recalculate solution state if we do
        new_solution.restore();

        new_solution
    }
}

fn optimize_route(route_ctx: &mut RouteContext, transport: &dyn TransportCost) {
    let path = route_to_path(route_ctx);

    // skip routes that are too small for optimization
    if path.len() <= 3 {
        return;
    }

    // build the adjacency matrix for LKH
    let adjacency = CostMatrix::new(route_ctx, transport);

    // run LKH algorithm and take last optimized path if it is different from original
    let Some(optimized) = lkh_optimize(adjacency, path.clone()).last().filter(|&optimized| *optimized != path).cloned()
    else {
        return;
    };

    rearrange_route(route_ctx, optimized);
}

/// Converts a [RouteContext] to a [Path] as vector of sequential indices.
fn route_to_path(route_ctx: &RouteContext) -> Path {
    (0..route_ctx.route().tour.total()).collect()
}

/// Reshufles [RouteContext] according to [Path] ordering.
fn rearrange_route(route_ctx: &mut RouteContext, mut path: Path) {
    let activities = route_ctx.route_mut().tour.activities_mut();
    let len = activities.len();

    // rearrange activities using swaps
    for i in (0..len).rev() {
        // current position in original ordering
        let current_idx = path[i];

        if current_idx != i {
            activities.swap(current_idx, i);
            let i_pos = path.iter().position(|&p| p == i).unwrap_or(i);
            path.swap(i, i_pos);
        }
    }
}

struct CostMatrix<'a> {
    profile: Profile,
    transport: &'a dyn TransportCost,
    neighbourhood: Vec<Vec<Node>>,
    locations: Vec<Location>,
}

impl<'a> CostMatrix<'a> {
    fn new(route_ctx: &RouteContext, transport: &'a dyn TransportCost) -> Self {
        let profile = route_ctx.route().actor.vehicle.profile.clone();

        let activities = route_ctx.route().tour.all_activities().collect::<Vec<_>>();
        let size = activities.len();

        // extract locations from activities
        let locations: Vec<Location> = activities.iter().map(|activity| activity.place.location).collect();

        // build neighborhood - for each node, store all other nodes sorted by dissimilarity metric (distance)
        let mut neighbourhood = Vec::with_capacity(size);

        for i in 0..size {
            let mut neighbors: Vec<(Node, Cost)> = (0..size)
                .filter(|&j| i != j)
                .map(|j| (j, transport.distance_approx(&profile, locations[i], locations[j])))
                .collect();

            neighbors.sort_by(|a, b| a.1.total_cmp(&b.1));
            neighbourhood[i] = neighbors.into_iter().map(|(node, _)| node).collect();
        }

        CostMatrix { profile, transport, neighbourhood, locations }
    }
}

impl<'a> AdjacencySpec for CostMatrix<'a> {
    fn cost(&self, edge: &Edge) -> Cost {
        let &(from, to) = edge;
        self.transport.distance_approx(&self.profile, self.locations[from], self.locations[to])
    }

    fn neighbours(&self, node: Node) -> &[Node] {
        self.neighbourhood[node].as_slice()
    }
}
