use super::*;
use crate::helpers::generate::create_test_job;

#[test]
fn can_generate_bounding_box() {
    let plan = Plan {
        jobs: vec![create_test_job(-1., 1.), create_test_job(1., 0.), create_test_job(3., 1.), create_test_job(1., 2.)],
        relations: None,
    };

    let ((min_lat, min_lng), (max_lat, max_lng)) = get_bounding_box_from_plan(&plan);

    assert_eq!(min_lat, -1.);
    assert_eq!(min_lng, 0.);
    assert_eq!(max_lat, 3.);
    assert_eq!(max_lng, 2.);
}
