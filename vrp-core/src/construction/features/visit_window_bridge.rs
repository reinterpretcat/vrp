//! Keeps a route's bridged visits honest once its untagged visits change.
//!
//! A bridged window reaches out to the untagged visits on the route (see `own_window_on`). Removing
//! such a visit can narrow the window under visits already served in the part it no longer covers.
//! After a ruin, this state takes those visits off the route and returns them for reinsertion, the
//! way breaks that lost their place are removed.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/visit_window_bridge_test.rs"]
mod visit_window_bridge_test;

use super::*;
use crate::construction::enablers::{IsAppointmentFn, own_window_on};
use crate::models::problem::{VisitWindowTagDimension, VisitWindowsDimension};

/// Creates a state-only feature that returns visits left outside their bridged window for reinsertion.
pub fn create_visit_window_bridge_feature(name: &str, is_visit: IsAppointmentFn) -> GenericResult<Feature> {
    FeatureBuilder::default().with_name(name).with_state(VisitWindowBridgeState { is_visit }).build()
}

struct VisitWindowBridgeState {
    is_visit: IsAppointmentFn,
}

impl VisitWindowBridgeState {
    /// The jobs on a route served outside every window of their chain, the own one bridged as the
    /// route stands now. Only routes whose shift has a bridged window are looked at.
    fn stranded(&self, route_ctx: &RouteContext) -> Vec<Job> {
        let route = route_ctx.route();
        let Some(windows) = route.actor.vehicle.dimens.get_visit_windows() else { return vec![] };
        if !windows.windows.values().any(|window| window.bridge) {
            return vec![];
        }

        route
            .tour
            .all_activities()
            .filter_map(|activity| {
                let single = activity.job.as_ref()?;
                let tag = single.dimens.get_visit_window_tag()?;
                let chain = windows.chain(tag);
                let (_, own) = chain.first()?;
                if !own.bridge || !(self.is_visit)(single) {
                    return None;
                }

                let departure = activity.schedule.departure;
                let start = departure - activity.place.duration;
                let fits = |(earliest, latest): (Timestamp, Timestamp)| start >= earliest && departure <= latest;
                let own_window = own_window_on(route, own, &self.is_visit);

                let is_inside =
                    fits(own_window) || chain.iter().skip(1).any(|(_, window)| fits((window.earliest, window.latest)));

                if is_inside { None } else { activity.retrieve_job() }
            })
            .collect()
    }
}

impl FeatureState for VisitWindowBridgeState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let locked = solution_ctx.locked.clone();

        let removed = solution_ctx
            .routes
            .iter_mut()
            .flat_map(|route_ctx| {
                let stranded =
                    self.stranded(route_ctx).into_iter().filter(|job| !locked.contains(job)).collect::<Vec<_>>();
                stranded.iter().for_each(|job| {
                    route_ctx.route_mut().tour.remove(job);
                });
                stranded
            })
            .collect::<Vec<_>>();

        solution_ctx.unassigned.extend(removed.into_iter().map(|job| (job, UnassignmentInfo::Unknown)));
    }
}
