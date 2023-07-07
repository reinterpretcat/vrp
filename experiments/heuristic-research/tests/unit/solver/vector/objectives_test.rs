use super::*;

const FITNESS_EPSILON: f64 = 1E-09;

#[test]
fn can_find_rosenbrock_optimum() {
    let rosenbrock_fn = get_fitness_fn_by_name("rosenbrock");

    assert!((rosenbrock_fn)(&[1., 1.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_rastrigin_optimum() {
    let rastrigin_fn = get_fitness_fn_by_name("rastrigin");

    assert!((rastrigin_fn)(&[0., 0.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_himmelblau_optimum() {
    let himmelblau_fn = get_fitness_fn_by_name("himmelblau");

    assert!((himmelblau_fn)(&[3., 2.]).abs() < FITNESS_EPSILON);
    assert!((himmelblau_fn)(&[-2.805118, 3.131312]).abs() < FITNESS_EPSILON);
    assert!((himmelblau_fn)(&[-3.779310, -3.283186]).abs() < FITNESS_EPSILON);
    assert!((himmelblau_fn)(&[3.584428, -1.848126]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_ackley_optimum() {
    let ackley_fn = get_fitness_fn_by_name("ackley");

    assert!((ackley_fn)(&[0., 0.]).abs() < FITNESS_EPSILON);
}

#[test]
fn can_find_matyas_optimum() {
    let matyas_fn = get_fitness_fn_by_name("matyas");

    assert!((matyas_fn)(&[0., 0.]).abs() < FITNESS_EPSILON);
}
