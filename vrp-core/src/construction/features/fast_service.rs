use super::*;
use crate::construction::enablers::calculate_travel;
use crate::models::solution::Activity;
use std::marker::PhantomData;

/// Creates a feature to prefer a fast servicing of jobs.
pub fn create_fast_service_feature<T: LoadOps>(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FastServiceObjective::<T>::new(transport, activity))
        .build()
}

struct FastServiceObjective<T> {
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> Objective for FastServiceObjective<T> {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| {
                let start_time = route_ctx.route().tour.start().unwrap().schedule.departure;
                route_ctx
                    .route()
                    .tour
                    .all_activities()
                    .filter(|a| self.is_static_delivery(a))
                    .map(move |a| a.schedule.departure - start_time)
            })
            .sum::<Duration>() as Cost
    }
}

impl<T: LoadOps> FeatureObjective for FastServiceObjective<T> {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        if let Some(cost) = self.estimate_job_service(move_ctx) {
            cost
        } else {
            Cost::default()
        }
    }
}

impl<T: LoadOps> FastServiceObjective<T> {
    fn new(transport: Arc<dyn TransportCost + Send + Sync>, activity: Arc<dyn ActivityCost + Send + Sync>) -> Self {
        Self { transport, activity, phantom: Default::default() }
    }

    fn estimate_job_service(&self, move_ctx: &MoveContext<'_>) -> Option<Cost> {
        let (route_ctx, activity_ctx) = match move_ctx {
            MoveContext::Route { .. } => return None,
            MoveContext::Activity { route_ctx, activity_ctx } => (route_ctx, activity_ctx),
        };

        let (_, (prev_to_tar_dur, tar_to_next_dur)) =
            calculate_travel(route_ctx, activity_ctx, self.transport.as_ref());

        // TODO add support for:
        //     - reloads
        //     - pickup jobs
        //     - p&d jobs

        // handle static delivery only
        if self.is_static_delivery(activity_ctx.target) {
            let start_time = route_ctx.route().tour.start().unwrap().schedule.departure;

            let arrival = activity_ctx.prev.schedule.departure + prev_to_tar_dur;
            let departure = self.activity.estimate_departure(route_ctx.route(), activity_ctx.target, arrival);
            let target_cost = departure - start_time;

            let next_delta = if let Some(next) = activity_ctx.next {
                let old_next_cost = next.schedule.arrival - start_time;

                let arrival = departure + tar_to_next_dur;
                let departure = self.activity.estimate_departure(route_ctx.route(), next, arrival);
                let new_next_cost = departure - start_time;

                new_next_cost - old_next_cost
            } else {
                Cost::default()
            };

            Some(target_cost + next_delta)
        } else {
            None
        }
    }

    fn is_static_delivery(&self, activity: &Activity) -> bool {
        activity
            .job
            .as_ref()
            .and_then::<&Demand<T>, _>(|job| job.dimens.get_demand())
            .map_or(false, |demand| demand.delivery.0.is_not_empty())
    }
}
