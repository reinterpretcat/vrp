use crate::helpers::construction::clustering::p;
use crate::helpers::solver::generate_matrix_distances_from_points;
use rosomaxa::prelude::Float;

pub fn create_test_distances() -> Vec<Float> {
    generate_matrix_distances_from_points(&[
        p(0., 0.), // A
        p(5., 5.),
        p(0., 10.),
        p(5., 15.),
        p(200., 200.), // Ghost
        p(25., 0.),    // B
        p(30., 5.),
        p(30., 10.),
    ])
}
