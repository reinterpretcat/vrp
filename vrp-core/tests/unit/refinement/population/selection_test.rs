use super::*;
use crate::helpers::refinement::population::*;
use std::f64::consts::PI;

struct Individuum(pub f64, pub f64);

fn fitness(individual: &Individuum) -> Tuple {
    const SCALE: f64 = 10.;

    let Individuum(r, h) = *individual;

    let sh = (r * r + h * h).sqrt();

    let s = (PI * r * sh) * SCALE;
    let t = PI * r * (r + sh) * SCALE;

    Tuple(s.round() as usize, t.round() as usize)
}

#[test]
fn test_selection_and_ranking() {
    let population = vec![
        Individuum(10.0, 19.61),
        Individuum(4.99, 5.10),
        Individuum(6.09, 0.79),
        Individuum(6.91, 10.62),
        Individuum(5.21, 18.87),
        Individuum(7.90, 8.98),
        Individuum(9.84, 0.78),
        Individuum(4.96, 0.60),
        Individuum(6.24, 19.66),
        Individuum(6.90, 15.09),
        Individuum(5.20, 18.86),
        Individuum(7.89, 8.97),
    ];
    let mo = MultiObjective::new(&[&Objective1, &Objective2]);

    // rate population (calculate fitness)
    let rated_population = population.iter().map(fitness).collect::<Vec<_>>();
    let ranked_population = select_and_rank(&rated_population, 7, &mo);

    let results = ranked_population.iter().map(|s| (s.index, s.rank)).collect::<Vec<_>>();

    assert_eq!(results.len(), 7);

    assert_eq!(results[0], (7, 0));

    assert_eq!(results[1], (1, 1));

    assert_eq!(results[2], (2, 2));

    assert_eq!(results[3], (10, 3));
    assert_eq!(results[4], (3, 3));

    assert_eq!(results[5], (4, 4));
    assert_eq!(results[6], (11, 4));
}
