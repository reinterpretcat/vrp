//! Provides a feature to minimize vehicle distance penalties.
//!
//! For each job on a route, the penalty is the excess distance from the job to its
//! assigned vehicle's start location compared to the nearest compatible vehicle's start.
//! penalty = max(0, dist(job, assigned_vehicle) - dist(job, nearest_compatible_vehicle))

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/vehicle_distance_test.rs"]
mod vehicle_distance_test;

use super::*;

custom_tour_state!(VehicleDistanceRouteData typeof RouteVehicleDistanceData);

/// A function type that checks whether a given actor is compatible with a given job.
pub type ActorJobCompatibilityFn = Arc<dyn Fn(&Job, &Actor) -> bool + Send + Sync>;

/// Route-level cached data for vehicle distance calculations.
#[derive(Clone, Default)]
pub struct RouteVehicleDistanceData {
    /// Penalty contribution from this route.
    pub penalty: Cost,
}

/// Provides a way to build a feature to minimize vehicle distance penalties.
pub struct VehicleDistanceFeatureBuilder {
    name: String,
    transport: Option<Arc<dyn TransportCost + Send + Sync>>,
    actors: Option<Vec<Arc<Actor>>>,
    compatibility_fn: Option<ActorJobCompatibilityFn>,
}

impl VehicleDistanceFeatureBuilder {
    /// Creates a new instance of `VehicleDistanceFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), transport: None, actors: None, compatibility_fn: None }
    }

    /// Sets the transport cost model.
    pub fn set_transport(mut self, transport: Arc<dyn TransportCost + Send + Sync>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Sets the fleet actors to consider when finding the nearest compatible vehicle.
    pub fn set_actors(mut self, actors: Vec<Arc<Actor>>) -> Self {
        self.actors = Some(actors);
        self
    }

    /// Sets the compatibility function that checks if an actor can serve a job.
    pub fn set_compatibility_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&Job, &Actor) -> bool + Send + Sync + 'static,
    {
        self.compatibility_fn = Some(Arc::new(func));
        self
    }

    /// Builds the feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let transport = self
            .transport
            .take()
            .ok_or_else(|| GenericError::from("transport must be set for vehicle_distance feature"))?;

        let actors =
            self.actors.take().ok_or_else(|| GenericError::from("actors must be set for vehicle_distance feature"))?;

        let compatibility_fn = self
            .compatibility_fn
            .take()
            .ok_or_else(|| GenericError::from("compatibility_fn must be set for vehicle_distance feature"))?;

        let shared = Arc::new(VehicleDistanceShared { transport, actors, compatibility_fn });

        let objective = VehicleDistanceObjective { shared: shared.clone() };
        let state = VehicleDistanceState { shared };

        FeatureBuilder::default().with_name(self.name.as_str()).with_objective(objective).with_state(state).build()
    }
}

/// Shared compute logic and dependencies for the vehicle-distance objective and state.
///
/// Both [`VehicleDistanceObjective`] and [`VehicleDistanceState`] go through the same
/// per-route penalty calculation; keeping it in one place avoids the trap of fixing
/// the formula in one copy and forgetting the other.
struct VehicleDistanceShared {
    transport: Arc<dyn TransportCost + Send + Sync>,
    actors: Vec<Arc<Actor>>,
    compatibility_fn: ActorJobCompatibilityFn,
}

impl VehicleDistanceShared {
    /// Round-trip distance between a depot and a job location for the given profile.
    ///
    /// Routing matrices are usually asymmetric (one-way streets, motorway ramps).
    /// Picking a single direction makes the "nearest vehicle" assignment depend on
    /// which way the matrix happens to favor; summing both directions gives a
    /// direction-neutral measure of how much travel a vehicle incurs to serve the
    /// job from its depot and return.
    fn round_trip(&self, profile: &Profile, depot: Location, job_loc: Location) -> Float {
        self.transport.distance_approx(profile, depot, job_loc)
            + self.transport.distance_approx(profile, job_loc, depot)
    }

    fn compute_route_penalty(&self, route_ctx: &RouteContext) -> Cost {
        let route = route_ctx.route();
        let profile = &route.actor.vehicle.profile;

        let assigned_start = match route.actor.detail.start.as_ref() {
            Some(start) => start.location,
            None => return 0.0,
        };

        let mut total_penalty = 0.0;

        for activity in route.tour.all_activities() {
            let Some(single) = activity.job.as_ref() else { continue };
            let job_loc = activity.place.location;
            let job = Job::Single(single.clone());

            let dist_assigned = self.round_trip(profile, assigned_start, job_loc);

            let dist_nearest = self.find_nearest_compatible_vehicle_dist(job_loc, &job, profile).unwrap_or(dist_assigned);

            let penalty = (dist_assigned - dist_nearest).max(0.0);
            total_penalty += penalty;
        }

        total_penalty
    }

    fn find_nearest_compatible_vehicle_dist(
        &self,
        job_loc: Location,
        job: &Job,
        profile: &Profile,
    ) -> Option<Float> {
        self.actors
            .iter()
            .filter(|actor| (self.compatibility_fn)(job, actor))
            .filter_map(|actor| actor.detail.start.as_ref().map(|s| s.location))
            .map(|start_loc| self.round_trip(profile, start_loc, job_loc))
            .min_by(|a, b| a.total_cmp(b))
    }
}

/// Gets the primary location of a job.
fn get_job_location(job: &Job) -> Option<Location> {
    match job {
        Job::Single(single) => single.places.first().and_then(|p| p.location),
        Job::Multi(multi) => multi.jobs.first().and_then(|s| s.places.first().and_then(|p| p.location)),
    }
}

struct VehicleDistanceObjective {
    shared: Arc<VehicleDistanceShared>,
}

impl FeatureObjective for VehicleDistanceObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        // We deliberately avoid `solution.solution.state.get_vehicle_distance_penalty()`
        // here: the cached value is maintained by `VehicleDistanceState` and is
        // sufficient for hot inner loops, but `fitness()` is the source of truth
        // reported back to the user. Summing the per-route cache directly keeps us
        // honest even if any pipeline ever desyncs the solution-level total.
        solution
            .solution
            .routes
            .iter()
            .map(|route_ctx| {
                route_ctx
                    .state()
                    .get_vehicle_distance_route_data()
                    .map(|data| data.penalty)
                    .unwrap_or_else(|| self.shared.compute_route_penalty(route_ctx))
            })
            .sum()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                let Some(job_loc) = get_job_location(job) else {
                    return Cost::default();
                };

                let route = route_ctx.route();
                let profile = &route.actor.vehicle.profile;

                let Some(assigned_start) = route.actor.detail.start.as_ref().map(|s| s.location) else {
                    return Cost::default();
                };

                let dist_assigned = self.shared.round_trip(profile, assigned_start, job_loc);

                let dist_nearest =
                    self.shared.find_nearest_compatible_vehicle_dist(job_loc, job, profile).unwrap_or(dist_assigned);

                (dist_assigned - dist_nearest).max(0.0)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct VehicleDistanceState {
    shared: Arc<VehicleDistanceShared>,
}

impl VehicleDistanceState {
    fn write_route_penalty(&self, route_ctx: &mut RouteContext) {
        let penalty = self.shared.compute_route_penalty(route_ctx);
        route_ctx.state_mut().set_vehicle_distance_route_data(RouteVehicleDistanceData { penalty });
    }

}

impl FeatureState for VehicleDistanceState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        // Eagerly refresh the affected route's penalty so the per-route cache is
        // always in sync with the tour. Stale-flag gating in accept_solution_state
        // was fragile because it relied on every caller to maintain `is_stale`
        // correctly across the construction and search pipelines.
        let route_ctx = solution_ctx.routes.get_mut(route_index).expect("route_index out of bounds");
        self.write_route_penalty(route_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        self.write_route_penalty(route_ctx);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        // Recompute every route's penalty regardless of the stale flag. This is the
        // canonical "rebuild from current tours" path and must not be skipped: at
        // every entry point that calls accept_solution_state (factories, restore,
        // finalize_insertion_ctx, search operators), tours may have been mutated
        // without our per-route cache being touched.
        solution_ctx.routes.iter_mut().for_each(|rc| self.write_route_penalty(rc));
    }
}
