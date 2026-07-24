//! Minimum-cost bipartite assignment (the Hungarian / Kuhn–Munkres algorithm).
//!
//! Given a square cost matrix, finds the perfect matching of rows to columns that minimizes the
//! total cost. Used to match drivers to derived territory seeds by commute distance.

#[cfg(test)]
#[path = "../../tests/unit/algorithms/assignment_test.rs"]
mod assignment_test;

/// Solves the minimum-cost assignment for a square `n×n` cost matrix using the O(n³) Kuhn–Munkres
/// algorithm with potentials. Returns `assign` where row `i` is matched to column `assign[i]`,
/// minimizing `Σ cost[i][assign[i]]`. An empty matrix returns an empty vector.
///
/// The matrix is assumed square (`cost.len() == cost[i].len()` for every row); callers building a
/// driver×seed matrix guarantee this because `k` seeds are derived from the driver count.
pub fn min_cost_assignment(cost: &[Vec<f64>]) -> Vec<usize> {
    let n = cost.len();
    if n == 0 {
        return Vec::new();
    }
    let m = cost[0].len();
    const INF: f64 = f64::INFINITY;

    // 1-indexed potentials and matching state (index 0 is the sentinel "unmatched" slot).
    let mut u = vec![0.0f64; n + 1];
    let mut v = vec![0.0f64; m + 1];
    let mut p = vec![0usize; m + 1]; // p[j] = row currently matched to column j (0 = none)
    let mut way = vec![0usize; m + 1];

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![INF; m + 1];
        let mut used = vec![false; m + 1];

        // Grow an augmenting path from row i until it reaches an unmatched column.
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = INF;
            let mut j1 = 0usize;

            for j in 1..=m {
                if !used[j] {
                    let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }

            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }

            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }

        // Flip the augmenting path, matching column j0 back to row i.
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    let mut assign = vec![0usize; n];
    for j in 1..=m {
        if p[j] != 0 {
            assign[p[j] - 1] = j - 1;
        }
    }
    assign
}
