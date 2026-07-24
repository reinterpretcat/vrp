use super::*;

/// A 2D fixture: `n×n` grid of jobs. Jobs in the bottom-left quadrant get `dense_value`, the rest
/// `sparse_value`. Returns the jobs `(index, value)` and a euclidean distance closure.
fn grid_fixture(
    n: usize,
    dense_value: f64,
    sparse_value: f64,
) -> (Vec<(usize, f64)>, impl Fn(usize, usize) -> f64 + Send + Sync + Clone) {
    let mut coords: Vec<(f64, f64)> = Vec::new();
    let mut jobs: Vec<(usize, f64)> = Vec::new();
    for gx in 0..n {
        for gy in 0..n {
            let idx = coords.len();
            coords.push((gx as f64 * 10.0, gy as f64 * 10.0));
            let v = if gx < n / 3 && gy < n / 3 { dense_value } else { sparse_value };
            jobs.push((idx, v));
        }
    }
    let d = move |a: usize, b: usize| {
        let (ax, ay) = coords[a];
        let (bx, by) = coords[b];
        ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
    };
    (jobs, d)
}

/// Assigns each job to the seed minimizing its power distance `dist − weight`.
fn owner(loc: usize, seeds: &[TerritorySeed], d: &impl Fn(usize, usize) -> f64) -> usize {
    (0..seeds.len())
        .min_by(|&a, &b| (d(loc, seeds[a].location) - seeds[a].weight).total_cmp(&(d(loc, seeds[b].location) - seeds[b].weight)))
        .unwrap()
}

#[test]
fn empty_or_zero_k_yields_no_seeds() {
    let noop = |_: usize, _: usize| 0.0;
    assert!(build_balanced_territory_seeds(&[], 3, noop, 10).is_empty());
    assert!(build_balanced_territory_seeds(&[(0, 1.0), (1, 1.0)], 0, noop, 10).is_empty());
}

#[test]
fn seeds_sit_on_existing_job_locations() {
    let (jobs, d) = grid_fixture(12, 1000.0, 50.0);
    let seeds = build_balanced_territory_seeds(&jobs, 12, d, 10);
    assert_eq!(seeds.len(), 12);
    let locations: std::collections::HashSet<usize> = jobs.iter().map(|(l, _)| *l).collect();
    for seed in &seeds {
        assert!(locations.contains(&seed.location), "seed {} is not a job location", seed.location);
    }
}

/// Regression for the weight-explosion bug: on spatially-skewed value, the clamp + value-aware
/// placement must keep every seed non-empty (no collapse) and cells compact (jobs stay near their
/// owner seed). Before the fix, ~half the seeds collapsed and the compactness ratio was >2.
#[test]
fn compact_and_no_collapse_on_skewed_value() {
    let (jobs, d) = grid_fixture(12, 1000.0, 50.0);
    let k = 12;
    let seeds = build_balanced_territory_seeds(&jobs, k, d.clone(), 20);
    assert_eq!(seeds.len(), k);

    let mut owned = vec![0usize; seeds.len()];
    let (mut sum_owner, mut sum_nearest) = (0.0f64, 0.0f64);
    for (loc, _) in &jobs {
        let o = owner(*loc, &seeds, &d);
        let n = (0..seeds.len()).min_by(|&a, &b| d(*loc, seeds[a].location).total_cmp(&d(*loc, seeds[b].location))).unwrap();
        owned[o] += 1;
        sum_owner += d(*loc, seeds[o].location);
        sum_nearest += d(*loc, seeds[n].location);
    }

    let collapsed = owned.iter().filter(|&&c| c == 0).count();
    assert_eq!(collapsed, 0, "no seed may collapse to an empty cell; owned={owned:?}");

    let compactness = sum_owner / sum_nearest.max(f64::EPSILON);
    assert!(compactness < 1.5, "cells must stay compact, got ratio {compactness:.2}");
}

/// Value balance: on a near-uniform value field the deterministic capacitated placement equalizes
/// per-cell value tightly — the richest cell carries under 1.5× the poorest — with every cell
/// non-empty. The derivation is deterministic (farthest-first init, no shared `create_kmedoids`), so
/// this is a stable, non-flaky bound. A 24×24 grid (≈48 jobs/cell) is used rather than a tiny grid:
/// with only a handful of jobs per cell, integer boundary effects alone force a ~2× split that no
/// balancer can beat, which says nothing about the algorithm.
#[test]
fn per_cell_value_is_balanced_on_uniform_field() {
    let (jobs, d) = grid_fixture(24, 100.0, 100.0);
    let seeds = build_balanced_territory_seeds(&jobs, 12, d.clone(), 20);

    let mut value = vec![0.0f64; seeds.len()];
    for (loc, v) in &jobs {
        value[owner(*loc, &seeds, &d)] += v;
    }
    let max = value.iter().copied().fold(f64::MIN, f64::max);
    let min = value.iter().copied().fold(f64::MAX, f64::min);
    assert!(min > 0.0 && max / min < 1.5, "per-cell value not balanced: min={min:.0} max={max:.0}");
}



