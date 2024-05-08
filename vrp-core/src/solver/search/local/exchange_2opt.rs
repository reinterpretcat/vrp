#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_2opt_test.rs"]
mod exchange_2opt_test;

use super::*;
use crate::construction::probing::*;
use crate::models::common::Distance;
use crate::models::problem::TravelTime;
use rosomaxa::utils::SelectionSamplingIterator;

/// Implements a classical TSP's two opt swap operation.
/// Please note that current implementation is not efficient with some features, such as tight
/// job's time windows, multi-jobs with restricted ordering (such as pickup & delivery), tour order, etc.
/// For algorithm details, see https://en.wikipedia.org/wiki/2-opt
#[derive(Default)]
pub struct ExchangeTwoOpt {}

impl LocalOperator for ExchangeTwoOpt {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        const ROUTES_ANALYZED_SIZE: usize = 4;

        let route_indices = SelectionSamplingIterator::new(
            0..insertion_ctx.solution.routes.len(),
            ROUTES_ANALYZED_SIZE,
            insertion_ctx.environment.random.clone(),
        );

        let mut opt_ctx = OptContext { insertion_ctx, new_insertion_ctx: None };
        for route_idx in route_indices {
            let (route_size, offset) = {
                let route = insertion_ctx.solution.routes.get(route_idx)?.route();
                (route.tour.total() as i32, if route.actor.detail.end.is_some() { 2 } else { 1 })
            };

            let mut has_improvement = true;
            while has_improvement {
                has_improvement = false;
                for i in 0..=(route_size - offset - 2) {
                    for j in (i + 1)..=(route_size - offset - 1) {
                        let delta = -opt_ctx.get_distance(route_idx, i, i + 1)
                            - opt_ctx.get_distance(route_idx, j, j + 1)
                            + opt_ctx.get_distance(route_idx, i, j)
                            + opt_ctx.get_distance(route_idx, i + 1, j + 1);

                        if delta < 0. {
                            has_improvement |= opt_ctx.apply_two_opt(route_idx, i, j);
                        }
                    }
                }
            }
        }

        opt_ctx.try_restore_solution()
    }
}

struct OptContext<'a> {
    insertion_ctx: &'a InsertionContext,
    new_insertion_ctx: Option<InsertionContext>,
}

impl<'a> OptContext<'a> {
    fn get_distance(&self, route_idx: usize, i: i32, j: i32) -> Distance {
        let transport = self.insertion_ctx.problem.transport.as_ref();
        let route_ctx = self
            .new_insertion_ctx
            .as_ref()
            .map(|insertion_ctx| &insertion_ctx.solution.routes)
            .unwrap_or_else(|| &self.insertion_ctx.solution.routes)
            .get(route_idx)
            .unwrap();
        let tour = &route_ctx.route().tour;

        let (i_loc, i_dep) = tour.get(i as usize).map(|a| (a.place.location, a.schedule.departure)).unwrap();
        let j_loc = tour.get(j as usize).unwrap().place.location;

        transport.distance(route_ctx.route(), i_loc, j_loc, TravelTime::Departure(i_dep))
    }

    fn apply_two_opt(&mut self, route_idx: usize, i: i32, j: i32) -> bool {
        let i = i as usize + 1;
        let j = j as usize;

        // NOTE do not apply two opt if there are locked jobs
        let route_ctx = self.insertion_ctx.solution.routes.get(route_idx).unwrap();
        let has_locked_jobs = route_ctx
            .route()
            .tour
            .activities_slice(i + 1, j)
            .iter()
            .filter_map(|a| a.retrieve_job())
            .any(|job| self.insertion_ctx.solution.locked.contains(&job));

        if has_locked_jobs || i == j {
            return false;
        }

        let new_insertion_ctx = if let Some(insertion_ctx) = self.new_insertion_ctx.as_mut() {
            insertion_ctx
        } else {
            self.new_insertion_ctx = Some(self.insertion_ctx.deep_copy());
            self.new_insertion_ctx.as_mut().unwrap()
        };

        let route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

        route_ctx.route_mut().tour.reverse(i, j);
        new_insertion_ctx.problem.goal.accept_route_state(route_ctx);

        true
    }

    fn try_restore_solution(self) -> Option<InsertionContext> {
        self.new_insertion_ctx.map(|new_insertion_ctx| {
            repair_solution_from_unknown(&new_insertion_ctx, || {
                InsertionContext::new(self.insertion_ctx.problem.clone(), self.insertion_ctx.environment.clone())
            })
        })
    }
}
