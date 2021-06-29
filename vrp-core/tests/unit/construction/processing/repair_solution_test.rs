use super::*;
use crate::helpers::construction::constraints::create_simple_demand;
use crate::helpers::models::problem::*;
use crate::models::common::*;
use crate::models::problem::{Jobs, ObjectiveCost, VehicleDetail, VehiclePlace};
use crate::models::Problem;

type JobData = (Option<Location>, (f64, f64), i32);
type VehicleData = ((Location, Option<f64>, Option<f64>), Option<(Location, Option<f64>, Option<f64>)>);

fn create_test_problem(
    singles: Vec<(&str, JobData)>,
    multies: Vec<(&str, Vec<JobData>)>,
    vehicles: Vec<(&str, VehicleData)>,
) -> Problem {
    let jobs = singles
        .into_iter()
        .map(|(id, (location, (tw_start, tw_end), demand))| {
            SingleBuilder::default()
                .id(id)
                .location(location)
                .times(vec![TimeWindow::new(tw_start, tw_end)])
                .demand(create_simple_demand(demand))
                .build_as_job_ref()
        })
        .chain(multies.into_iter().map(|(id, singles)| {
            let singles = singles
                .into_iter()
                .map(|(location, (tw_start, tw_end), demand)| {
                    SingleBuilder::default()
                        .id(id)
                        .location(location)
                        .times(vec![TimeWindow::new(tw_start, tw_end)])
                        .demand(create_simple_demand(demand))
                        .build()
                })
                .collect();

            MultiBuilder::default().id(id).jobs(singles).build()
        }))
        .collect::<Vec<_>>();

    let vehicles = vehicles
        .into_iter()
        .map(|(id, ((start_location, start_earliest, start_latest), end))| {
            VehicleBuilder::default()
                .id(id)
                .details(vec![VehicleDetail {
                    start: Some(VehiclePlace {
                        location: start_location,
                        time: TimeInterval { earliest: start_earliest, latest: start_latest },
                    }),
                    end: end.map(|(end_location, end_earliest, end_latest)| VehiclePlace {
                        location: end_location,
                        time: TimeInterval { earliest: end_earliest, latest: end_latest },
                    }),
                }])
                .build()
        })
        .collect::<Vec<_>>();

    let fleet = Arc::new(FleetBuilder::default().add_driver(test_driver()).add_vehicles(vehicles).build());

    let transport = TestTransportCost::new_shared();

    Problem {
        fleet: fleet.clone(),
        jobs: Arc::new(Jobs::new(&fleet, jobs, &transport)),
        locks: vec![],
        constraint: Arc::new(Default::default()),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        objective: Arc::new(ObjectiveCost::default()),
        extras: Arc::new(Default::default()),
    }
}

#[test]
fn can_restore_solution() {
    unimplemented!()
}
