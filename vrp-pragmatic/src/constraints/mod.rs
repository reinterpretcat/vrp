use std::cmp::Ordering::Less;
use std::sync::Arc;
use vrp_core::construction::constraints::{TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use vrp_core::construction::heuristics::SolutionContext;
use vrp_core::models::common::{Dimensions, IdDimension, ValueDimension};
use vrp_core::models::problem::{Costs, Single};
use vrp_core::models::solution::{Activity, Route};

fn as_single_job<F>(activity: &Activity, condition: F) -> Option<&Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| if condition(job) { Some(job) } else { None })
}

fn get_shift_index(dimens: &Dimensions) -> usize {
    *dimens.get_value::<usize>("shift_index").unwrap()
}

fn get_vehicle_id_from_job(job: &Arc<Single>) -> Option<&String> {
    job.dimens.get_value::<String>("vehicle_id")
}

fn is_correct_vehicle(route: &Route, target_id: &String, target_shift: usize) -> bool {
    route.actor.vehicle.dimens.get_id().unwrap() == target_id
        && get_shift_index(&route.actor.vehicle.dimens) == target_shift
}

fn get_max_cost(solution_ctx: &SolutionContext) -> f64 {
    let get_total_cost = |costs: &Costs, distance: f64, duration: f64| {
        costs.fixed
            + costs.per_distance * distance
            + costs.per_driving_time.max(costs.per_service_time).max(costs.per_waiting_time) * duration
    };

    solution_ctx
        .routes
        .iter()
        .map(|rc| {
            let distance = rc.state.get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
            let duration = rc.state.get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

            get_total_cost(&rc.route.actor.vehicle.costs, distance, duration)
                + get_total_cost(&rc.route.actor.driver.costs, distance, duration)
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(0.)
}

mod breaks;
pub use self::breaks::BreakModule;

mod work_balance;
pub use self::work_balance::*;

mod priorities;
pub use self::priorities::PriorityModule;

mod reloads;
pub use self::reloads::ReloadMultiTrip;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::SkillsModule;
