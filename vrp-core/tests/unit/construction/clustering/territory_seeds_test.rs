use super::*;

fn dist(a: usize, b: usize) -> f64 {
    (a as f64 - b as f64).abs()
}

/// Index of the seed minimizing a job's power distance `dist − weight`. `seeds` is `(loc, weight)`.
fn assign(loc: usize, seeds: &[(usize, f64)]) -> usize {
    (0..seeds.len())
        .min_by(|&a, &b| (dist(loc, seeds[a].0) - seeds[a].1).total_cmp(&(dist(loc, seeds[b].0) - seeds[b].1)))
        .unwrap()
}

/// Per-seed value spread (max − min) under a `(loc, weight)` assignment. `jobs` is `(loc, value, load)`.
fn value_spread(jobs: &[(usize, f64, f64)], seeds: &[(usize, f64)]) -> f64 {
    let mut val = vec![0.0f64; seeds.len()];
    for (loc, v, _) in jobs {
        val[assign(*loc, seeds)] += v;
    }
    val.iter().copied().fold(f64::MIN, f64::max) - val.iter().copied().fold(f64::MAX, f64::min)
}

/// The largest per-seed load under a `(loc, weight)` assignment. `jobs` is `(loc, value, load)`.
fn max_cell_load(jobs: &[(usize, f64, f64)], seeds: &[(usize, f64)]) -> f64 {
    let mut load = vec![0.0f64; seeds.len()];
    for (loc, _, l) in jobs {
        load[assign(*loc, seeds)] += l;
    }
    load.iter().copied().fold(f64::MIN, f64::max)
}

#[test]
fn empty_or_zero_k_yields_no_seeds() {
    assert!(build_balanced_territory_seeds(&[], 3, f64::INFINITY, dist, 10).is_empty());
    assert!(build_balanced_territory_seeds(&[(0, 1.0, 0.0), (1, 1.0, 0.0)], 0, f64::INFINITY, dist, 10).is_empty());
}

#[test]
fn seeds_sit_on_existing_job_locations() {
    let jobs: Vec<(usize, f64, f64)> = (0..12).map(|i| (i, 1.0, 0.0)).collect();
    let seeds = build_balanced_territory_seeds(&jobs, 3, f64::INFINITY, dist, 10);
    assert_eq!(seeds.len(), 3);
    let locations: std::collections::HashSet<usize> = jobs.iter().map(|(l, _, _)| *l).collect();
    for seed in &seeds {
        assert!(locations.contains(&seed.location), "seed {} is not a job location", seed.location);
    }
}

#[test]
fn value_balancing_tightens_the_spread() {
    // A continuum 0..11 with high value on the left, low on the right: the pure-distance boundary
    // (~midpoint) leaves the left cell far richer. Weights should shift the boundary left and even
    // out the value. No load, so capacity is irrelevant (INFINITY).
    let jobs: Vec<(usize, f64, f64)> = (0..12).map(|i| (i, if i < 6 { 10.0 } else { 1.0 }, 0.0)).collect();

    let seeds = build_balanced_territory_seeds(&jobs, 2, f64::INFINITY, dist, 30);
    assert_eq!(seeds.len(), 2);

    let weighted: Vec<(usize, f64)> = seeds.iter().map(|s| (s.location, s.weight)).collect();
    let unweighted: Vec<(usize, f64)> = seeds.iter().map(|s| (s.location, 0.0)).collect();

    assert!(
        value_spread(&jobs, &weighted) < value_spread(&jobs, &unweighted),
        "weights should tighten the value spread"
    );
}

#[test]
fn feasibility_guard_shrinks_the_overloaded_cell() {
    // Uniform value (value balance alone splits 6/6), but the left jobs are load-heavy. Value-only
    // (INFINITY capacity) leaves the left cell far over capacity; the guard shrinks it, dropping the
    // peak cell load — feasibility dominates value balance.
    let jobs: Vec<(usize, f64, f64)> = (0..12).map(|i| (i, 1.0, if i < 6 { 10.0 } else { 1.0 })).collect();

    let value_only = build_balanced_territory_seeds(&jobs, 2, f64::INFINITY, dist, 30);
    let feasible = build_balanced_territory_seeds(&jobs, 2, 30.0, dist, 30);

    let vo: Vec<(usize, f64)> = value_only.iter().map(|s| (s.location, s.weight)).collect();
    let fe: Vec<(usize, f64)> = feasible.iter().map(|s| (s.location, s.weight)).collect();

    assert!(
        max_cell_load(&jobs, &fe) < max_cell_load(&jobs, &vo),
        "feasibility guard should reduce the overloaded cell: feasible={}, value_only={}",
        max_cell_load(&jobs, &fe),
        max_cell_load(&jobs, &vo)
    );
}
