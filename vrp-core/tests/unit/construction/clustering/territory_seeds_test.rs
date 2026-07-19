use super::*;

fn dist(a: usize, b: usize) -> f64 {
    (a as f64 - b as f64).abs()
}

/// Per-seed value spread (max − min) when each job is assigned to the seed minimizing its power
/// distance `dist − weight`. `seeds` is `(location, weight)`.
fn value_spread(jobs: &[(usize, f64)], seeds: &[(usize, f64)]) -> f64 {
    let mut val = vec![0.0f64; seeds.len()];
    for (loc, v) in jobs {
        let i = (0..seeds.len())
            .min_by(|&a, &b| {
                (dist(*loc, seeds[a].0) - seeds[a].1).total_cmp(&(dist(*loc, seeds[b].0) - seeds[b].1))
            })
            .unwrap();
        val[i] += v;
    }
    let max = val.iter().copied().fold(f64::MIN, f64::max);
    let min = val.iter().copied().fold(f64::MAX, f64::min);
    max - min
}

#[test]
fn empty_or_zero_k_yields_no_seeds() {
    assert!(build_balanced_territory_seeds(&[], 3, dist, 10).is_empty());
    assert!(build_balanced_territory_seeds(&[(0, 1.0), (1, 1.0)], 0, dist, 10).is_empty());
}

#[test]
fn seeds_sit_on_existing_job_locations() {
    let jobs: Vec<(usize, f64)> = (0..12).map(|i| (i, 1.0)).collect();
    let seeds = build_balanced_territory_seeds(&jobs, 3, dist, 10);
    assert_eq!(seeds.len(), 3);
    let locations: std::collections::HashSet<usize> = jobs.iter().map(|(l, _)| *l).collect();
    for seed in &seeds {
        assert!(locations.contains(&seed.location), "seed {} is not a job location", seed.location);
    }
}

#[test]
fn value_balancing_tightens_the_spread() {
    // A continuum 0..11 with high value on the left, low on the right: the pure-distance boundary
    // (~midpoint) leaves the left cell far richer. Weights should shift the boundary left and even
    // out the value. Jobs straddle the boundary, so balancing has something to move.
    let jobs: Vec<(usize, f64)> = (0..12).map(|i| (i, if i < 6 { 10.0 } else { 1.0 })).collect();

    let seeds = build_balanced_territory_seeds(&jobs, 2, dist, 30);
    assert_eq!(seeds.len(), 2);

    let weighted: Vec<(usize, f64)> = seeds.iter().map(|s| (s.location, s.weight)).collect();
    let unweighted: Vec<(usize, f64)> = seeds.iter().map(|s| (s.location, 0.0)).collect();

    let weighted_spread = value_spread(&jobs, &weighted);
    let unweighted_spread = value_spread(&jobs, &unweighted);

    assert!(
        weighted_spread < unweighted_spread,
        "weights should tighten the value spread: weighted={weighted_spread}, unweighted={unweighted_spread}"
    );
}
