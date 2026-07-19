//! Derives balanced, compact territory seeds from customer jobs via a **capacitated Lloyd**
//! relaxation: `k` medoids positioned so that assigning each job to its nearest seed yields cells
//! of ~equal production value that are also spatially compact (non-overlapping). The seeds feed the
//! territory objective's anchors; the driver→seed matching is decided separately (Hungarian).
//!
//! Distance-only k-medoids gives compact but wildly unequal-value cells (value follows job count);
//! adding per-seed power weights to rebalance value was found to *break* compactness (large weights
//! stretch cells until their hulls overlap). Balancing value at **placement** time instead keeps
//! both: the nearest-seed partition is already balanced, so no weights are needed (they stay `0`).

#[cfg(test)]
#[path = "../../../tests/unit/construction/clustering/territory_seeds_test.rs"]
mod territory_seeds_test;

use crate::algorithms::clustering::kmedoids::create_kmedoids;
use std::collections::HashSet;

/// A derived territory: a medoid `location` (an existing job location, so the distance matrix is
/// reused) and its power `weight` (currently always `0.0` — value balance comes from placement, not
/// weights; the field is retained for the territory objective's power-distance mechanism).
#[derive(Clone, Copy, Debug)]
pub struct TerritorySeed {
    /// The seed location — a medoid, i.e. one of the input job locations.
    pub location: usize,
    /// The power-distance weight `w_i`. `0.0` for derived seeds (balance is done at placement time).
    pub weight: f64,
}

/// Builds `k` balanced, compact territory seeds from `jobs` (each `(location, production_value)`).
///
/// Seeds are placed by a capacitated Lloyd relaxation ([`balanced_value_medoids`]) so the
/// nearest-seed partition is both value-balanced and compact; the returned weights are `0.0`.
/// `iterations` bounds the Lloyd passes.
pub fn build_balanced_territory_seeds(
    jobs: &[(usize, f64)],
    k: usize,
    distance_fn: impl Fn(usize, usize) -> f64 + Send + Sync,
    iterations: usize,
) -> Vec<TerritorySeed> {
    if k == 0 || jobs.is_empty() {
        return Vec::new();
    }

    // Medoids are drawn from the distinct job locations.
    let locations: Vec<usize> = {
        let mut seen = HashSet::new();
        jobs.iter().map(|(l, _)| *l).filter(|l| seen.insert(*l)).collect()
    };

    // Fewer distinct locations than territories: one seed per location.
    if k >= locations.len() {
        return locations.into_iter().map(|location| TerritorySeed { location, weight: 0.0 }).collect();
    }

    balanced_value_medoids(jobs, &locations, k, &distance_fn, iterations)
        .into_iter()
        .map(|location| TerritorySeed { location, weight: 0.0 })
        .collect()
}

/// Places `k` medoids so each cell carries ~equal production value AND stays spatially compact — a
/// capacitated Lloyd relaxation. It alternates an **equal-value assignment** (each job to its
/// nearest seed that still has value room, most spatially-constrained jobs first) with a **medoid
/// recompute** per cell. After convergence the equal-value partition ≈ the nearest-seed partition,
/// so no rebalancing weights are needed. Returns the medoid locations (each an existing job
/// location).
fn balanced_value_medoids(
    jobs: &[(usize, f64)],
    locations: &[usize],
    k: usize,
    distance_fn: &(impl Fn(usize, usize) -> f64 + Send + Sync),
    iterations: usize,
) -> Vec<usize> {
    let mut seeds: Vec<usize> = create_kmedoids(locations, k, |a, b| distance_fn(*a, *b)).keys().copied().collect();
    let kk = seeds.len();
    if kk == 0 {
        return seeds;
    }
    let total_value: f64 = jobs.iter().map(|(_, v)| *v).sum();
    let cap = (total_value / kk as f64).max(f64::EPSILON);

    for _ in 0..iterations {
        // Assign the most spatially-constrained jobs first (largest gap between nearest and
        // second-nearest seed), so a job with a clear home claims it before capacity fills.
        let mut order: Vec<usize> = (0..jobs.len()).collect();
        let regret = |ji: usize| -> f64 {
            let loc = jobs[ji].0;
            let (mut d0, mut d1) = (f64::MAX, f64::MAX);
            for &s in &seeds {
                let d = distance_fn(loc, s);
                if d < d0 {
                    d1 = d0;
                    d0 = d;
                } else if d < d1 {
                    d1 = d;
                }
            }
            d1 - d0
        };
        order.sort_by(|&a, &b| regret(b).total_cmp(&regret(a)));

        let mut cell_value = vec![0.0f64; kk];
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); kk];
        for &ji in &order {
            let (loc, val) = jobs[ji];
            let mut cand: Vec<usize> = (0..kk).collect();
            cand.sort_by(|&a, &b| distance_fn(loc, seeds[a]).total_cmp(&distance_fn(loc, seeds[b])));
            let chosen = cand.iter().copied().find(|&c| cell_value[c] + val <= cap).unwrap_or(cand[0]);
            cell_value[chosen] += val;
            members[chosen].push(loc);
        }

        // Recompute each cell's medoid: the member location minimizing total distance to the rest.
        for (c, cell) in members.iter().enumerate() {
            if cell.is_empty() {
                continue;
            }
            let best = cell
                .iter()
                .copied()
                .min_by(|&x, &y| {
                    let sx: f64 = cell.iter().map(|&m| distance_fn(x, m)).sum();
                    let sy: f64 = cell.iter().map(|&m| distance_fn(y, m)).sum();
                    sx.total_cmp(&sy)
                })
                .unwrap();
            seeds[c] = best;
        }
    }
    seeds
}
