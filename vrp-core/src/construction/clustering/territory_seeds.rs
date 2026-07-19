//! Derives balanced territory seeds from customer jobs: distance-compact medoids whose per-seed
//! *weights* rebalance the power-cell assignment (`argmin_i dist(j, seed_i) − w_i`) toward equal
//! total production value. The seeds feed the territory objective's anchors and the weights feed
//! its power-distance overlap penalty; the driver→seed matching is decided separately (Hungarian).

#[cfg(test)]
#[path = "../../../tests/unit/construction/clustering/territory_seeds_test.rs"]
mod territory_seeds_test;

use crate::algorithms::clustering::kmedoids::create_kmedoids;
use std::collections::HashSet;

/// A derived territory: a medoid `location` (an existing job location, so the distance matrix is
/// reused) and its value-balancing power `weight`.
#[derive(Clone, Copy, Debug)]
pub struct TerritorySeed {
    /// The seed location — a medoid, i.e. one of the input job locations.
    pub location: usize,
    /// The power-distance weight `w_i`: larger enlarges this seed's cell.
    pub weight: f64,
}

/// Builds `k` balanced territory seeds from `jobs` (each `(location, production_value, load)`).
///
/// Seeds are distance-compact medoids from [`create_kmedoids`]; their weights are then tuned over
/// `iterations` of power-cell balancing:
///
/// `w_i += lr_value · (value_target − value_i) − lr_load · max(0, load_i − capacity)`
///
/// The first term equalizes production value (`value_target = Σvalue / seeds`). The second is a
/// feasibility guard: a cell whose estimated `load` (e.g. summed service time) exceeds `capacity`
/// is shrunk regardless of value, so **feasibility dominates value balance**. Pass
/// `capacity = f64::INFINITY` to disable the guard and balance on value alone. Balancing only moves
/// jobs across boundaries where they actually straddle two seeds.
pub fn build_balanced_territory_seeds(
    jobs: &[(usize, f64, f64)],
    k: usize,
    capacity: f64,
    distance_fn: impl Fn(usize, usize) -> f64 + Send + Sync,
    iterations: usize,
) -> Vec<TerritorySeed> {
    if k == 0 || jobs.is_empty() {
        return Vec::new();
    }

    // Medoids are drawn from the distinct job locations.
    let locations: Vec<usize> = {
        let mut seen = HashSet::new();
        jobs.iter().map(|(l, _, _)| *l).filter(|l| seen.insert(*l)).collect()
    };

    // Fewer distinct locations than territories: one seed per location, nothing to balance.
    if k >= locations.len() {
        return locations.into_iter().map(|location| TerritorySeed { location, weight: 0.0 }).collect();
    }

    let medoids = create_kmedoids(&locations, k, |a, b| distance_fn(*a, *b));
    let seeds: Vec<usize> = medoids.keys().copied().collect();
    if seeds.is_empty() {
        return Vec::new();
    }

    let total_value: f64 = jobs.iter().map(|(_, v, _)| *v).sum();
    let value_target = total_value / seeds.len() as f64;

    let nearest = |loc: usize, weights: &[f64]| -> usize {
        (0..seeds.len())
            .min_by(|&a, &b| {
                (distance_fn(loc, seeds[a]) - weights[a]).total_cmp(&(distance_fn(loc, seeds[b]) - weights[b]))
            })
            .unwrap_or(0)
    };
    // A distance-unit step size: the mean job→nearest-seed distance (at zero weights).
    let zero = vec![0.0; seeds.len()];
    let scale = (jobs.iter().map(|(loc, _, _)| distance_fn(*loc, seeds[nearest(*loc, &zero)])).sum::<f64>()
        / jobs.len() as f64)
        .max(f64::EPSILON);
    // Convert value / load units into distance units. `capacity == INFINITY` ⇒ `lr_load == 0`, so
    // the feasibility term vanishes and this reduces to pure value balancing.
    let lr_value = scale / value_target.max(f64::EPSILON);
    let lr_load = scale / capacity.max(f64::EPSILON);

    let mut weights = vec![0.0f64; seeds.len()];
    for _ in 0..iterations {
        let mut value_per_seed = vec![0.0f64; seeds.len()];
        let mut load_per_seed = vec![0.0f64; seeds.len()];
        for (loc, val, load) in jobs.iter() {
            let i = nearest(*loc, &weights);
            value_per_seed[i] += *val;
            load_per_seed[i] += *load;
        }
        for i in 0..seeds.len() {
            let overload = (load_per_seed[i] - capacity).max(0.0);
            weights[i] += lr_value * (value_target - value_per_seed[i]) - lr_load * overload;
        }
    }

    seeds.into_iter().zip(weights).map(|(location, weight)| TerritorySeed { location, weight }).collect()
}
