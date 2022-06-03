use super::*;
use std::ops::Deref;

const FITNESS_EPSILON: f64 = 1E-09;

#[test]
fn can_find_rosenbrock_optimum() {
    let rosenbrock = get_fitness_fn_by_name("rosenbrock");
    let rosenbrock = rosenbrock.deref();

    assert!(rosenbrock(&[1., 1.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_rastrigin_optimum() {
    let rastrigin = get_fitness_fn_by_name("rastrigin");
    let rastrigin = rastrigin.deref();

    assert!(rastrigin(&[0., 0.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_himmelblau_optimum() {
    let himmelblau = get_fitness_fn_by_name("himmelblau");
    let himmelblau = himmelblau.deref();

    assert!(himmelblau(&[3., 2.]).abs() < FITNESS_EPSILON);
    assert!(himmelblau(&[-2.805118, 3.131312]).abs() < FITNESS_EPSILON);
    assert!(himmelblau(&[-3.779310, -3.283186]).abs() < FITNESS_EPSILON);
    assert!(himmelblau(&[3.584428, -1.848126]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_ackley_optimum() {
    let ackley = get_fitness_fn_by_name("ackley");
    let ackley = ackley.deref();

    assert!(ackley(&[0., 0.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_matyas_optimum() {
    let matyas = get_fitness_fn_by_name("matyas");
    let matyas = matyas.deref();

    assert!(matyas(&[0., 0.]).abs() < FITNESS_EPSILON);
}
