use super::*;
use crate::helpers::{create_single_with_location, get_costs};
use vrp_core::models::common::SingleDimLoad;

#[test]
fn can_handle_reload_jobs_with_merge() {
    let create_reload = || {
        let mut single = create_single_with_location(None);
        single.dimens.set_value("type", "reload".to_string());

        Job::Single(Arc::new(single))
    };
    let create_job = || Job::Single(Arc::new(create_single_with_location(None)));
    let (transport, activity) = get_costs();
    let multi_trip = Arc::new(ReloadMultiTrip::new(activity, transport, Box::new(|_| SingleDimLoad::default())));
    let constraint = CapacityConstraintModule::<SingleDimLoad>::new_with_multi_trip(2, multi_trip);

    assert_eq!(constraint.merge(create_reload(), create_job()).map(|_| ()), Err(2));
    assert_eq!(constraint.merge(create_job(), create_reload()).map(|_| ()), Err(2));
    assert_eq!(constraint.merge(create_reload(), create_reload()).map(|_| ()), Err(2));
}
