use super::*;

fn just_noise(
    probability: f64,
    range: (f64, f64),
    random: Arc<dyn Random + Send + Sync>,
) -> VectorHeuristicOperatorMode {
    VectorHeuristicOperatorMode::JustNoise(Noise::new_with_ratio(probability, range, random))
}

fn dimen_noise(
    probability: f64,
    range: (f64, f64),
    dimen: usize,
    random: Arc<dyn Random + Send + Sync>,
) -> VectorHeuristicOperatorMode {
    let dimen = vec![dimen].into_iter().collect();
    VectorHeuristicOperatorMode::DimensionNoise(Noise::new_with_ratio(probability, range, random), dimen)
}

#[test]
pub fn can_create_and_use_rosenbrock_function_2d() {
    let function_fn = create_rosenbrock_function();

    assert_eq!((function_fn)(&[2., 2.]), 401.);
    assert_eq!((function_fn)(&[1., 1.]), 0.);
    assert_eq!((function_fn)(&[0.5, 0.5]), 6.5);
    assert_eq!((function_fn)(&[0., 0.]), 1.);
    assert_eq!((function_fn)(&[-0.5, -0.5]), 58.5);
    assert_eq!((function_fn)(&[-1., -1.]), 404.);
    assert_eq!((function_fn)(&[-2., -2.]), 3609.);
}

#[test]
fn can_solve_rosenbrock() {
    let random = Arc::new(DefaultRandom::default());
    let (solutions, _) = Solver::default()
        .with_fitness_fn(create_rosenbrock_function())
        .with_init_solutions(vec![vec![2., 2.]])
        .with_search_operator(just_noise(1., (-0.05, 0.05), random.clone()), "first", 1.)
        .with_search_operator(just_noise(1., (0., 0.1), random.clone()), "second", 0.25)
        .with_search_operator(just_noise(1., (-0.1, 0.), random.clone()), "third", 0.25)
        .with_search_operator(dimen_noise(1., (-0.1, 0.1), 0, random.clone()), "fourth", 0.5)
        .with_search_operator(dimen_noise(1., (-0.1, 0.1), 1, random.clone()), "five", 0.25)
        .with_diversify_operator(dimen_noise(1., (-0.5, 0.5), 1, random))
        .with_termination(Some(5), Some(1000), None, None)
        .solve()
        .expect("cannot build and use solver");

    assert_eq!(solutions.len(), 1);
    let (_, fitness) = solutions.first().unwrap();
    assert!(*fitness < 0.01);
}
