use crate::helpers::algorithms::p;
use crate::solver::mutation::ruin::cluster_removal::get_max_curvature;

#[test]
fn can_get_max_curvature() {
    let values =
        &[p(0., 0.), p(1., 0.25), p(2., 0.5), p(3., 0.75), p(4., 1.), p(6., 2.), p(7., 4.), p(8., 6.), p(9., 8.)];

    assert_eq!(get_max_curvature(values), 2.);
}
