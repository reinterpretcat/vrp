#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_2opt_test.rs"]
mod exchange_2opt_test;

use super::*;
use crate::construction::probing::*;
use crate::models::common::Distance;
use crate::models::problem::TravelTime;
use hashbrown::HashSet;

/// Implements a classical TSP's two opt swap operation.
/// See https://en.wikipedia.org/wiki/2-opt
#[derive(Default)]
pub struct ExchangeTwoOpt {}

impl LocalOperator for ExchangeTwoOpt {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let route_idx = get_random_route_idx(insertion_ctx)?;
        let (route_size, offset) = {
            let route = &insertion_ctx.solution.routes.get(route_idx)?.route;
            (route.tour.total() as i32, if route.actor.detail.end.is_some() { 2 } else { 1 })
        };

        let mut opt_ctx = OptContext { insertion_ctx, new_insertion_ctx: None, route_idx };
        for i in 0..=(route_size - offset - 2) {
            for j in (i + 1)..=(route_size - offset - 1) {
                let delta = -opt_ctx.get_distance(i, i + 1) - opt_ctx.get_distance(j, j + 1)
                    + opt_ctx.get_distance(i, j)
                    + opt_ctx.get_distance(i + 1, j + 1);

                if delta < 0. {
                    opt_ctx.apply_two_opt(i, j);
                }
            }
        }

        opt_ctx.try_restore_solution()
    }
}

struct OptContext<'a> {
    insertion_ctx: &'a InsertionContext,
    new_insertion_ctx: Option<InsertionContext>,
    route_idx: usize,
}

impl<'a> OptContext<'a> {
    fn get_distance(&self, i: i32, j: i32) -> Distance {
        let transport = self.insertion_ctx.problem.transport.as_ref();
        let route_ctx = self
            .new_insertion_ctx
            .as_ref()
            .map(|insertion_ctx| &insertion_ctx.solution.routes)
            .unwrap_or_else(|| &self.insertion_ctx.solution.routes)
            .get(self.route_idx)
            .unwrap();
        let tour = &route_ctx.route.tour;

        let (i_loc, i_dep) = tour.get(i as usize).map(|a| (a.place.location, a.schedule.departure)).unwrap();
        let j_loc = tour.get(j as usize).unwrap().place.location;

        transport.distance(route_ctx.route.as_ref(), i_loc, j_loc, TravelTime::Departure(i_dep))
    }

    fn apply_two_opt(&mut self, i: i32, j: i32) {
        let i = i as usize + 1;
        let j = j as usize;

        // NOTE do not apply two opt if there are locked jobs
        let route_ctx = self.insertion_ctx.solution.routes.get(self.route_idx).unwrap();
        if route_ctx
            .route
            .tour
            .activities_slice(i, j)
            .iter()
            .filter_map(|a| a.retrieve_job())
            .any(|job| self.insertion_ctx.solution.locked.contains(&job))
        {
            return;
        }

        let new_insertion_ctx = if let Some(insertion_ctx) = self.new_insertion_ctx.as_mut() {
            insertion_ctx
        } else {
            self.new_insertion_ctx = Some(self.insertion_ctx.deep_copy());
            self.new_insertion_ctx.as_mut().unwrap()
        };

        let route_ctx = new_insertion_ctx.solution.routes.get_mut(self.route_idx).unwrap();

        route_ctx.route_mut().tour.reverse(i, j);
        new_insertion_ctx.problem.goal.accept_route_state(route_ctx);
    }

    fn try_restore_solution(self) -> Option<InsertionContext> {
        self.new_insertion_ctx.map(|mut new_insertion_ctx| {
            let route_ctx = new_insertion_ctx.solution.routes.get(self.route_idx).unwrap().deep_copy();
            let locked = &new_insertion_ctx.solution.locked;

            // NOTE try_repair_route requires an empty route (but locked jobs should stay)
            let empty_route_ctx = new_insertion_ctx.solution.routes.get_mut(self.route_idx).unwrap();
            route_ctx.route.tour.jobs().filter(|job| locked.get(job).is_none()).for_each(|job| {
                empty_route_ctx.route_mut().tour.remove(&job);
            });
            new_insertion_ctx.problem.goal.accept_route_state(empty_route_ctx);

            let mut assigned_jobs = empty_route_ctx.route.tour.jobs().collect::<HashSet<_>>();
            let unassigned = try_repair_route(&mut new_insertion_ctx, &mut assigned_jobs, &route_ctx);

            finalize_synchronization(&mut new_insertion_ctx, self.insertion_ctx, unassigned.into_iter().collect());

            new_insertion_ctx
        })
    }
}
