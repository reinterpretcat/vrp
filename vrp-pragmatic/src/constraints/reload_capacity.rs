use crate::constraints::*;
use std::collections::HashSet;
use std::iter::once;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::common::{Cost, IdDimension, ValueDimension};
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::Activity;

pub struct ReloadCapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>,
    state_keys: Vec<i32>,
    capacity: CapacityConstraintModule<Capacity>,
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ReloadCapacityConstraintModule<Capacity>
{
    pub fn new(code: i32, cost_reward: Cost, threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> Self {
        let capacity_constraint = CapacityConstraintModule::<Capacity>::new(code);
        let hard_route_constraint = capacity_constraint
            .get_constraints()
            .filter_map(|c| match c {
                ConstraintVariant::HardRoute(c) => Some(c.clone()),
                _ => None,
            })
            .next()
            .unwrap();

        let hard_activity_constraint = capacity_constraint
            .get_constraints()
            .filter_map(|c| match c {
                ConstraintVariant::HardActivity(c) => Some(c.clone()),
                _ => None,
            })
            .next()
            .unwrap();

        Self {
            threshold,
            state_keys: capacity_constraint
                .state_keys()
                .chain(vec![HAS_RELOAD_KEY, MAX_TOUR_LOAD_KEY].iter())
                .cloned()
                .collect(),
            capacity: capacity_constraint,
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::SoftRoute(Arc::new(ReloadSoftRouteConstraint { cost: cost_reward })),
                ConstraintVariant::HardRoute(Arc::new(ReloadHardRouteConstraint { code, hard_route_constraint })),
                ConstraintVariant::HardActivity(Arc::new(ReloadHardActivityConstraint::<Capacity> {
                    code,
                    hard_activity_constraint,
                    phantom: PhantomData,
                })),
            ],
        }
    }

    fn is_vehicle_full(rc: &RouteContext, threshold: &Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> bool {
        let tour = &rc.route.tour;
        let state = &rc.state;

        if let Some(end) = tour.end() {
            let empty_capacity = Capacity::default();
            let max_capacity = threshold(rc.route.actor.vehicle.dimens.get_capacity().unwrap());

            let load = *state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).unwrap_or_else(|| &empty_capacity);

            load >= max_capacity
        } else {
            false
        }
    }

    fn recalculate_states(ctx: &mut RouteContext) {
        let (route, state) = ctx.as_mut();
        let demand = Demand::<Capacity>::default();

        let last_idx = route.tour.total() - 1;
        let (_, _, starts) = (0_usize..).zip(route.tour.all_activities()).fold(
            (Capacity::default(), Capacity::default(), Vec::<(usize, usize, Capacity)>::new()),
            |(start_total, end_total, mut acc), (idx, a)| {
                let demand = CapacityConstraintModule::<Capacity>::get_demand(a).unwrap_or(&demand);
                let (start_total, end_total) = if as_reload_job(a).is_some() || idx == last_idx {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
                    let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

                    acc.push((start_idx, end_idx, start_total));

                    (end_total, Capacity::default())
                } else {
                    (start_total + demand.delivery.0, end_total + demand.pickup.1 - demand.delivery.1)
                };

                (start_total, end_total, acc)
            },
        );

        let ends = starts.iter().cloned().fold(vec![], |mut acc, (start_idx, end_idx, total)| {
            let (current, _) =
                route.tour.activities_slice(start_idx, end_idx).iter().fold((total, total), |(current, max), a| {
                    CapacityConstraintModule::<Capacity>::store_max_past_current_state(state, a, current, max)
                });

            acc.push(current);

            acc
        });

        state.put_route_state::<Capacity>(MAX_TOUR_LOAD_KEY, *ends.iter().max().unwrap());

        starts.into_iter().zip(ends.into_iter()).for_each(|((start_idx, end_idx, _), end)| {
            route
                .tour
                .activities_slice(start_idx, end_idx)
                .iter()
                .rev()
                .fold(end, |max, a| CapacityConstraintModule::<Capacity>::store_max_future_state(state, a, max));
        });
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for ReloadCapacityConstraintModule<Capacity>
{
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Job) {
        if is_reload_job(job) {
            route_ctx.state_mut().put_route_state(HAS_RELOAD_KEY, true);
            // move all unassigned reloads back to ignored
            let jobs = get_reload_jobs(route_ctx, &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|i| !jobs.contains(i));
            solution_ctx.ignored.extend(jobs.into_iter());

            self.accept_route_state(route_ctx);
        } else {
            self.accept_route_state(route_ctx);
            if Self::is_vehicle_full(route_ctx, &self.threshold) {
                // move all reloads for this shift to required
                let jobs = get_reload_jobs(route_ctx, &solution_ctx.ignored)
                    .chain(get_reload_jobs(route_ctx, &solution_ctx.required))
                    .collect::<HashSet<_>>();

                solution_ctx.ignored.retain(|i| !jobs.contains(i));
                solution_ctx.locked.extend(jobs.iter().cloned());
                solution_ctx.required.extend(jobs.into_iter());
            }
        }

        remove_trivial_reloads(solution_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        if has_reload_index(ctx) {
            Self::recalculate_states(ctx);
        } else {
            self.capacity.accept_route_state(ctx);
        }
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
        remove_trivial_reloads(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct ReloadSoftRouteConstraint {
    cost: Cost,
}

impl SoftRouteConstraint for ReloadSoftRouteConstraint {
    fn estimate_job(&self, ctx: &RouteContext, job: &Job) -> f64 {
        if is_reload_job(job) {
            -ctx.route.actor.vehicle.costs.fixed - self.cost
        } else {
            0.
        }
    }
}

/// Locks reload jobs to specific vehicles
struct ReloadHardRouteConstraint {
    code: i32,
    hard_route_constraint: Arc<dyn HardRouteConstraint + Send + Sync>,
}

impl HardRouteConstraint for ReloadHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if is_reload_job(job) {
            let job = job.to_single();
            let vehicle_id = get_vehicle_id_from_job(&job).unwrap();
            let shift_index = get_shift_index(&job.dimens);

            return if !is_correct_vehicle(ctx, vehicle_id, shift_index) {
                Some(RouteConstraintViolation { code: self.code })
            } else {
                None
            };
        }

        if has_reload_index(ctx) {
            // TODO we need to check all ranges to avoid going into full scan when vehicle is full
            //  store reload jobs in route state and call tour.index to scan each reload range?
            None
        } else {
            self.hard_route_constraint.evaluate_job(ctx, job)
        }
    }
}

struct ReloadHardActivityConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    hard_activity_constraint: Arc<dyn HardActivityConstraint + Send + Sync>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardActivityConstraint for ReloadHardActivityConstraint<Capacity>
{
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(_) = as_reload_job(activity_ctx.target) {
            // NOTE insert reload job in route only as last

            let is_first = activity_ctx.prev.job.is_none();
            let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

            return if is_first || is_not_last {
                Some(ActivityConstraintViolation { code: self.code, stopped: false })
            } else {
                None
            };
        }

        let has_reload = has_reload_index(route_ctx);

        if has_reload {
            let multi = activity_ctx.target.retrieve_job().and_then(|job| match &job {
                Job::Multi(multi) => Some((job.clone(), multi.jobs.len())),
                _ => None,
            });

            if let Some((job, singles)) = multi {
                let processed_activities = route_ctx.route.tour.job_activities(&job).count();
                // NOTE check capacity violation for reloads
                // we allow temporary overload when pickups are inserted, but delivery should fix it later
                if processed_activities == singles - 1 {
                    let capacity: Capacity = *route_ctx.route.actor.vehicle.dimens.get_capacity().unwrap();
                    let index = route_ctx.route.tour.activity_index(activity_ctx.prev).unwrap();

                    // TODO optimize this?
                    let has_violation = route_ctx.route.tour.activities_slice(0, index).iter().rev().any(|a| {
                        *route_ctx.state.get_activity_state::<Capacity>(MAX_PAST_CAPACITY_KEY, a).unwrap() > capacity
                    });

                    if has_violation {
                        return Some(ActivityConstraintViolation { code: self.code, stopped: false });
                    }
                }
            }
        }

        // TODO optimize: we should skip activity range till next reload if "stopped:true" is returned
        if let Some(result) = self.hard_activity_constraint.evaluate_activity(route_ctx, activity_ctx) {
            let stopped = result.stopped && !has_reload;
            Some(ActivityConstraintViolation { code: self.code, stopped })
        } else {
            None
        }
    }
}

/// Removes reloads at the start and end of tour.
fn remove_trivial_reloads(ctx: &mut SolutionContext) {
    if ctx.required.is_empty() {
        ctx.routes.iter_mut().for_each(|rc| {
            let activities = rc.route.tour.total();
            let first_reload_idx = 1_usize;
            let last_reload_idx = if rc.route.actor.detail.end.is_some() { activities - 2 } else { activities - 1 };

            once(first_reload_idx).chain(once(last_reload_idx)).for_each(|idx| {
                if as_reload_job(rc.route.tour.get(idx).unwrap()).is_some() {
                    rc.route_mut().tour.remove_activity_at(idx);
                }
            });
        });
    }
}

/// Creates job transition which removes reload jobs from required and adds them to locked.
fn create_job_transition() -> Box<dyn JobContextTransition + Send + Sync> {
    Box::new(ConcreteJobContextTransition {
        remove_required: |_, job| is_reload_job(job),
        promote_required: |_, _| false,
        remove_locked: |_, _| false,
        promote_locked: |_, job| is_reload_job(job),
    })
}

fn is_reload_single(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn is_reload_job(job: &Job) -> bool {
    job.as_single().map_or(false, |single| is_reload_single(single))
}

fn as_reload_job(activity: &Activity) -> Option<&Arc<Single>> {
    as_single_job(activity, |job| is_reload_single(job))
}

fn has_reload_index(ctx: &RouteContext) -> bool {
    *ctx.state.get_route_state::<bool>(HAS_RELOAD_KEY).unwrap_or(&false)
}

fn get_reload_jobs<'a>(route_ctx: &'a RouteContext, collection: &'a Vec<Job>) -> Box<dyn Iterator<Item = Job> + 'a> {
    let shift_index = get_shift_index(&route_ctx.route.actor.vehicle.dimens);
    let vehicle_id = route_ctx.route.actor.vehicle.dimens.get_id().unwrap();

    Box::new(
        collection
            .iter()
            .filter(move |job| match job {
                Job::Single(job) => {
                    is_reload_single(&job)
                        && get_shift_index(&job.dimens) == shift_index
                        && get_vehicle_id_from_job(&job).unwrap() == vehicle_id
                }
                _ => false,
            })
            .cloned(),
    )
}
