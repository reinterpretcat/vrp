extern crate rayon;

use self::rayon::prelude::*;

use crate::construction::states::{InsertionContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::{Actor, Job, TransportCost};
use crate::models::solution::TourActivity;
use crate::refinement::ruin::Ruin;
use crate::refinement::RefinementContext;
use hashbrown::HashMap;
use std::cmp::Ordering::Greater;
use std::iter::once;
use std::sync::Arc;

/// Detects the most cost expensive jobs in each route and delete them with their neighbours
struct RuinWorstJobs {
    transport: Arc<dyn TransportCost + Send + Sync>,
    range: (i32, i32),
}

impl Ruin for RuinWorstJobs {
    fn run(&self, _refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let locked = insertion_ctx.locked.clone();
        let random = insertion_ctx.random.clone();

        let mut route_jobs = self.get_route_jobs(&insertion_ctx.solution);
        let routes_savings = self.get_routes_cost_savings(&insertion_ctx.solution);

        routes_savings.iter().for_each(|(rc, savings)| {
            let worst = savings.iter().filter(|(job, _)| !locked.contains(job)).next();
            if let Some((job, _)) = worst {
                let remove = random.uniform_int(self.range.0, self.range.1) as usize;
                once(job.clone())
                    .chain(problem.jobs.neighbors(
                        rc.route.actor.vehicle.profile,
                        &job,
                        Default::default(),
                        std::f64::MAX,
                    ))
                    .take(remove)
                    .for_each(|job| {
                        // NOTE actual insertion context modification via route mut
                        route_jobs
                            .get_mut(&job)
                            .unwrap_or_else(|| panic!("Cannot get route for job"))
                            .route_mut()
                            .tour
                            .remove(&job);
                    });
            }
        });

        insertion_ctx
    }
}

impl RuinWorstJobs {
    fn get_route_jobs(&self, solution: &SolutionContext) -> HashMap<Arc<Job>, RouteContext> {
        solution.routes.iter().fold(HashMap::default(), |acc, rc| {
            rc.route.tour.jobs().fold(acc, |mut acc, job| {
                acc.insert(job, rc.clone());
                acc
            })
        })
    }

    fn get_routes_cost_savings(&self, solution: &SolutionContext) -> Vec<(RouteContext, Vec<(Arc<Job>, Cost)>)> {
        solution
            .routes
            .par_iter()
            .map(|rc| {
                let actor = rc.route.actor.as_ref();
                let mut savings: Vec<(Arc<Job>, Cost)> = rc
                    .route
                    .tour
                    .all_activities()
                    .as_slice()
                    .windows(3)
                    .fold(HashMap::<Arc<Job>, Cost>::default(), |mut acc, iter| match iter {
                        [start, eval, end] => {
                            let savings = self.get_cost_savings(actor, start, eval, end);
                            let job = eval.retrieve_job().unwrap_or_else(|| panic!("Unexpected activity without job"));
                            *acc.entry(job).or_insert(0.) += savings;

                            acc
                        }
                        _ => panic!("Unexpected activity window"),
                    })
                    .drain()
                    .collect();
                savings.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Greater));

                (rc.clone(), savings)
            })
            .collect()
    }

    fn get_cost_savings(&self, actor: &Actor, start: &TourActivity, middle: &TourActivity, end: &TourActivity) -> Cost {
        self.get_cost(actor, start, middle) + self.get_cost(actor, middle, end) - self.get_cost(actor, start, end)
    }

    fn get_cost(&self, actor: &Actor, from: &TourActivity, to: &TourActivity) -> Cost {
        self.transport.cost(actor, from.place.location, to.place.location, from.schedule.departure)
    }
}
