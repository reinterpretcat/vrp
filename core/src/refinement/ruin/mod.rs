use crate::construction::states::{InsertionContext, RouteContext};
use crate::models::problem::Job;
use crate::models::Problem;
use crate::refinement::RefinementContext;
use crate::utils::Random;
use std::iter::{empty, once};
use std::sync::Arc;

/// Specifies ruin strategy.
pub trait Ruin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod adjusted_string_removal;

pub use self::adjusted_string_removal::AdjustedStringRemoval;

mod neighbour_removal;

pub use self::neighbour_removal::NeighbourRemoval;

mod random_route_removal;

pub use self::random_route_removal::RandomRouteRemoval;

mod random_job_removal;

pub use self::random_job_removal::RandomJobRemoval;

mod worst_jobs_removal;

pub use self::worst_jobs_removal::WorstJobRemoval;

/// Provides the way to run multiple ruin methods.
pub struct CompositeRuin {
    ruins: Vec<Vec<(Arc<dyn Ruin>, f64)>>,
    weights: Vec<usize>,
}

impl Default for CompositeRuin {
    fn default() -> Self {
        let adjusted_string_default = Arc::new(AdjustedStringRemoval::default());
        let adjusted_string_agressive = Arc::new(AdjustedStringRemoval::new(30, 120, 0.02));

        let neighbour_removal = Arc::new(NeighbourRemoval::new(10, 30, 0.5));
        let neighbour_agressive = Arc::new(NeighbourRemoval::new(30, 120, 0.5));

        let worst_job_default = Arc::new(WorstJobRemoval::default());
        let random_job_default = Arc::new(RandomJobRemoval::default());
        let random_route_default = Arc::new(RandomRouteRemoval::default());

        Self::new(vec![
            (
                vec![
                    (adjusted_string_default.clone(), 1.),
                    (random_route_default.clone(), 0.05),
                    (random_job_default.clone(), 0.05),
                ],
                100,
            ),
            (vec![(adjusted_string_agressive.clone(), 1.)], 3),
            (
                vec![
                    (neighbour_removal.clone(), 1.),
                    (random_route_default.clone(), 0.05),
                    (random_job_default.clone(), 0.05),
                ],
                25,
            ),
            (vec![(neighbour_agressive.clone(), 1.)], 2),
            (vec![(random_job_default.clone(), 1.), (random_route_default.clone(), 0.02)], 2),
            (vec![(random_route_default.clone(), 1.), (random_job_default.clone(), 0.02)], 2),
            (vec![(worst_job_default.clone(), 1.), (adjusted_string_default.clone(), 1.)], 1),
        ])
    }
}

impl CompositeRuin {
    pub fn new(ruins: Vec<(Vec<(Arc<dyn Ruin>, f64)>, usize)>) -> Self {
        let mut ruins = ruins;
        ruins.sort_by(|(_, a), (_, b)| b.cmp(&a));

        let weights = ruins.iter().map(|(_, weight)| *weight).collect();

        Self { ruins: ruins.into_iter().map(|(ruin, _)| ruin).collect(), weights }
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.random.clone();

        let index = insertion_ctx.random.weighted(self.weights.iter());

        let mut insertion_ctx = self
            .ruins
            .get(index)
            .unwrap()
            .iter()
            .filter(|(_, probability)| *probability > random.uniform_real(0., 1.))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}

fn get_chunk_size(ctx: &InsertionContext, range: &(usize, usize), threshold: f64) -> usize {
    let &(min, max) = range;

    let assigned = ctx.problem.jobs.size() - ctx.solution.unassigned.len() - ctx.solution.ignored.len();

    let max_limit = (assigned as f64 * threshold).min(max as f64).round() as usize;

    ctx.random.uniform_int(min as i32, max as i32).min(max_limit as i32) as usize
}

/// Returns randomly selected job within all its neighbours.
fn select_seed_jobs<'a>(
    problem: &'a Problem,
    routes: &[RouteContext],
    random: &Arc<dyn Random + Send + Sync>,
) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
    let seed = select_seed_job(routes, random);

    if let Some((route_index, job)) = seed {
        return Box::new(once(job.clone()).chain(problem.jobs.neighbors(
            routes.get(route_index).unwrap().route.actor.vehicle.profile,
            &job,
            Default::default(),
            std::f64::MAX,
        )));
    }

    Box::new(empty())
}

/// Selects seed job from existing solution
fn select_seed_job<'a>(
    routes: &'a [RouteContext],
    random: &Arc<dyn Random + Send + Sync>,
) -> Option<(usize, Arc<Job>)> {
    if routes.is_empty() {
        return None;
    }

    let route_index = random.uniform_int(0, (routes.len() - 1) as i32) as usize;
    let mut ri = route_index;

    loop {
        let rc = routes.get(ri).unwrap();

        if rc.route.tour.has_jobs() {
            let job = select_random_job(rc, random);
            if let Some(job) = job {
                return Some((route_index, job));
            }
        }

        ri = (ri + 1) % routes.len();
        if ri == route_index {
            break;
        }
    }

    None
}

fn select_random_job(rc: &RouteContext, random: &Arc<dyn Random + Send + Sync>) -> Option<Arc<Job>> {
    let size = rc.route.tour.activity_count();
    if size == 0 {
        return None;
    }

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = rc.route.tour.get(ai).and_then(|a| a.retrieve_job());

        if job.is_some() {
            return job;
        }

        ai = (ai + 1) % (size + 1);
        if ai == activity_index {
            break;
        }
    }

    None
}
