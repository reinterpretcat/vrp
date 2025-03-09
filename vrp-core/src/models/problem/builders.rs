//! Provides a way to build some of the core models using the builder pattern.

use crate::construction::features::{JobDemandDimension, VehicleCapacityDimension};
use crate::models::common::{Cost, Demand, Dimensions, Duration, LoadOps, Location, TimeSpan, TimeWindow, Timestamp};
use crate::models::problem::{
    Costs, Job, JobIdDimension, JobPermutation, Multi, Place, Single, Vehicle, VehicleDetail, VehicleIdDimension,
    VehiclePlace,
};
use rosomaxa::prelude::{GenericError, GenericResult};
use std::sync::Arc;

/// Provides a way to build a [Single] job using the builder pattern.
#[derive(Debug)]
pub struct SingleBuilder(Single);

impl Default for SingleBuilder {
    fn default() -> Self {
        Self(Single { places: vec![], dimens: Default::default() })
    }
}

impl SingleBuilder {
    /// Adds a new place to single job's `places` collection. Use this api to add multiple places
    /// which are used as alternative places (e.g. locations) to serve the job.
    pub fn add_place(mut self, place: Place) -> Self {
        self.0.places.push(place);
        self
    }

    /// Adds new places to single job's `places` collection.
    pub fn add_places(mut self, places: impl Iterator<Item = Place>) -> Self {
        self.0.places.extend(places);
        self
    }

    /// Sets a job id dimension.
    pub fn id(mut self, id: &str) -> Self {
        self.0.dimens.set_job_id(id.to_string());
        self
    }

    /// A simple api to set a single job's demand.
    pub fn demand<T: LoadOps>(mut self, demand: Demand<T>) -> Self {
        self.0.dimens.set_job_demand(demand);
        self
    }

    /// A simple api to associate arbitrary property within the job.
    pub fn dimension(mut self, func: impl FnOnce(&mut Dimensions)) -> Self {
        func(&mut self.0.dimens);
        self
    }

    /// A simple api to set location of the first place.
    /// Normally, location is represented as an index in routing matrix.
    /// Fails if used with more than one place, creates a new place if no places are specified.
    pub fn location(mut self, location: Location) -> GenericResult<Self> {
        self.ensure_single_place()?.location = Some(location);
        Ok(self)
    }

    /// A simple api to set duration of the first place.
    /// Fails if used with more than one place, creates a new place if no places are specified.
    pub fn duration(mut self, duration: Duration) -> GenericResult<Self> {
        self.ensure_single_place()?.duration = duration;
        Ok(self)
    }

    /// A simple api to set time windows of the first place.
    /// Fails if used with more than one place, creates a new place if no places are specified.
    pub fn times(mut self, times: Vec<TimeWindow>) -> GenericResult<Self> {
        self.ensure_single_place()?.times = times.into_iter().map(TimeSpan::Window).collect();
        Ok(self)
    }

    /// Builds a [Single] job.
    pub fn build(self) -> GenericResult<Single> {
        Ok(self.0)
    }

    /// Builds a [Job].
    pub fn build_as_job(self) -> GenericResult<Job> {
        Ok(Job::Single(Arc::new(self.0)))
    }

    fn ensure_single_place(&mut self) -> GenericResult<&mut Place> {
        if self.0.places.len() > 1 {
            return Err("cannot use the simple api with multiple places, use `SingleBuilder::add_place` and `JobPlaceBuilder` instead".into());
        }

        if self.0.places.is_empty() {
            self.0.places.push(empty_place());
        }

        self.0.places.first_mut().ok_or_else(|| GenericError::from("no places"))
    }
}

/// Provides a way to build a [Place] used internally by [Single] job.
pub struct JobPlaceBuilder(Place);

impl Default for JobPlaceBuilder {
    fn default() -> Self {
        Self(empty_place())
    }
}

impl JobPlaceBuilder {
    /// Sets place's location.
    pub fn location(mut self, loc: Option<Location>) -> Self {
        self.0.location = loc;
        self
    }

    /// Sets place's duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.0.duration = duration;
        self
    }

    /// Sets place's time windows.
    pub fn times(mut self, times: Vec<TimeWindow>) -> Self {
        self.0.times = times.into_iter().map(TimeSpan::Window).collect();
        self
    }

    /// Builds a job [Place].
    pub fn build(self) -> GenericResult<Place> {
        Ok(self.0)
    }
}

/// Provides a way to build a [Multi] job using the builder pattern.
#[derive(Default)]
pub struct MultiBuilder {
    jobs: Vec<Arc<Single>>,
    dimens: Dimensions,
    permutator: Option<Box<dyn JobPermutation>>,
}

impl MultiBuilder {
    /// Sets a job id dimension.
    pub fn id(mut self, id: &str) -> Self {
        self.dimens.set_job_id(id.to_string());
        self
    }

    /// Adds a [Single] as sub-job.
    pub fn add_job(mut self, single: Single) -> Self {
        self.jobs.push(Arc::new(single));
        self
    }

    /// A simple api to associate arbitrary property within the job.
    pub fn dimension(mut self, func: impl FnOnce(&mut Dimensions)) -> Self {
        func(&mut self.dimens);
        self
    }

    /// Sets a permutation logic which tells allowed order of sub-jobs assignment.
    /// If omitted, sub-jobs can be assigned only in the order of addition.
    pub fn permutation(mut self, permutation: impl JobPermutation + 'static) -> Self {
        self.permutator = Some(Box::new(permutation));
        self
    }

    /// Builds [Multi] job as shared reference.
    pub fn build(self) -> GenericResult<Arc<Multi>> {
        if self.jobs.len() < 2 {
            return Err("the number of sub-jobs must be 2 or more".into());
        }

        Ok(match self.permutator {
            Some(permutator) => Multi::new_shared_with_permutator(self.jobs, self.dimens, permutator),
            _ => Multi::new_shared(self.jobs, self.dimens),
        })
    }

    /// Builds a [Job].
    pub fn build_as_job(self) -> GenericResult<Job> {
        Ok(Job::Multi(self.build()?))
    }
}

fn empty_place() -> Place {
    // NOTE a time window must be present as it is expected in evaluator logic.
    Place { location: None, duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::max())] }
}

/// Provides a way to build a [Vehicle].
pub struct VehicleBuilder(Vehicle);

impl Default for VehicleBuilder {
    fn default() -> Self {
        Self(Vehicle {
            profile: Default::default(),
            costs: Costs {
                fixed: 0.0,
                per_distance: 1.,
                per_driving_time: 0.0,
                per_waiting_time: 0.0,
                per_service_time: 0.0,
            },
            dimens: Default::default(),
            details: vec![],
        })
    }
}

impl VehicleBuilder {
    /// Sets a vehicle id dimension.
    pub fn id(mut self, id: &str) -> Self {
        self.0.dimens.set_vehicle_id(id.to_string());
        self
    }

    /// Adds a vehicle detail which specifies start/end location, time, etc.
    /// Use [VehicleDetailBuilder] to construct one.
    pub fn add_detail(mut self, detail: VehicleDetail) -> Self {
        self.0.details.push(detail);
        self
    }

    /// Sets routing profile index which is used to configure which routing data to use within the vehicle.
    pub fn set_profile_idx(mut self, idx: usize) -> Self {
        self.0.profile.index = idx;
        self
    }

    /// Sets a cost per distance unit.
    pub fn set_distance_cost(mut self, cost: Cost) -> Self {
        self.0.costs.per_distance = cost;
        self
    }

    /// Sets a cost per duration unit.
    pub fn set_duration_cost(mut self, cost: Cost) -> Self {
        self.0.costs.per_driving_time = cost;
        self.0.costs.per_service_time = cost;
        self.0.costs.per_waiting_time = cost;
        self
    }

    /// Sets a vehicle capacity dimension.
    pub fn capacity<T: LoadOps>(mut self, value: T) -> Self {
        self.0.dimens.set_vehicle_capacity(value);
        self
    }

    /// A simple api to associate arbitrary property within the vehicle.
    pub fn dimension(mut self, func: impl FnOnce(&mut Dimensions)) -> Self {
        func(&mut self.0.dimens);
        self
    }

    /// Builds a [Vehicle].
    pub fn build(self) -> GenericResult<Vehicle> {
        if self.0.details.is_empty() {
            Err("at least one vehicle detail needs to be added, use `VehicleDetailBuilder` and `add_detail` function"
                .into())
        } else {
            Ok(self.0)
        }
    }
}

/// Provides a way to build [VehicleDetail].
pub struct VehicleDetailBuilder(VehicleDetail);

impl Default for VehicleDetailBuilder {
    fn default() -> Self {
        Self(VehicleDetail { start: None, end: None })
    }
}

impl VehicleDetailBuilder {
    /// Sets start location.
    pub fn set_start_location(mut self, location: Location) -> Self {
        self.ensure_start().location = location;
        self
    }

    /// Sets earliest departure time for start location.
    pub fn set_start_time(mut self, earliest: Timestamp) -> Self {
        self.ensure_start().time.earliest = Some(earliest);
        // NOTE disable departure time optimization
        self.ensure_start().time.latest = Some(earliest);
        self
    }

    /// Sets a latest departure time to enable departure time optimization (disabled implicitly with `set_start_time` call).
    pub fn set_start_time_latest(mut self, latest: Timestamp) -> Self {
        self.ensure_start().time.latest = Some(latest);
        self
    }

    /// Sets end location.
    pub fn set_end_location(mut self, location: Location) -> Self {
        self.ensure_end().location = location;
        self
    }

    /// Sets the latest arrival time for end location.
    pub fn set_end_time(mut self, latest: Timestamp) -> Self {
        self.ensure_end().time.latest = Some(latest);
        self
    }

    fn ensure_start(&mut self) -> &mut VehiclePlace {
        if self.0.start.is_none() {
            self.0.start = Some(VehiclePlace { location: 0, time: Default::default() });
        }
        self.0.start.as_mut().unwrap()
    }

    fn ensure_end(&mut self) -> &mut VehiclePlace {
        if self.0.end.is_none() {
            self.0.end = Some(VehiclePlace { location: 0, time: Default::default() });
        }
        self.0.end.as_mut().unwrap()
    }

    /// Builds vehicle detail.
    pub fn build(self) -> GenericResult<VehicleDetail> {
        if self.0.start.is_none() { Err("start place must be defined for vehicle detail".into()) } else { Ok(self.0) }
    }
}
