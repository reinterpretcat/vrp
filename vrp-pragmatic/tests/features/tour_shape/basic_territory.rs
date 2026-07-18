//! E2E tests for the `territory` objective: it should form balanced, capacity-aware territories
//! around a per-driver anchor. These tests build a real pragmatic `Problem`, run the actual
//! metaheuristic solver (not cheapest-insertion), and check the resulting solution against
//! meaningful (non-trivial) balance/territory tolerances -- see the module doc comment on each
//! helper for what would make the assertion fail if the objective regressed.

use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::{Solution, Tour};
use crate::format::CoordIndex;
use crate::helpers::*;
use std::collections::HashMap;
use vrp_core::prelude::Float;

// region: fixture construction

/// A built problem plus the ground truth needed to grade the solution: which vehicle each job
/// "belongs" to by construction (its nearest anchor), the production value it was given, and the
/// full vehicle id roster (so a driver that ends up completely unused -- zero jobs, no tour in the
/// solution at all -- is still counted as a zero, not silently dropped from the balance stats).
struct TerritoryFixture {
    problem: Problem,
    home_vehicle: HashMap<String, String>,
    job_value: HashMap<String, Float>,
    vehicle_ids: Vec<String>,
}

/// Resolves `loc`'s routing-matrix index the same way `create_matrix_from_problem` will, so the
/// `territory` objective's `anchors` map (which is expressed in matrix indices) can be built
/// before the final `Problem` (with its `objectives`) exists. `CoordIndex::new` only reads
/// `plan`/`fleet`, so a probe `Problem` with `objectives: None` gives the same indices the real
/// problem will get. `loc` must already be a job or vehicle-shift location in `plan`/`fleet`
/// (typically a vehicle's shift-end location, which is how these fixtures place anchors).
fn location_index(plan: &Plan, fleet: &Fleet, loc: (f64, f64)) -> usize {
    let probe = Problem { plan: plan.clone(), fleet: fleet.clone(), objectives: None };
    CoordIndex::new(&probe)
        .get_by_loc(&loc.to_loc())
        .unwrap_or_else(|| panic!("location {loc:?} is not indexed by the problem"))
}

fn territory_objective(proximity: TerritoryProximity, balance: BalancePeriodMetric, anchors: HashMap<String, usize>) -> Objective {
    Territory { proximity, balance: Some(balance), anchors }
}

/// Two drivers sharing a start location at the origin, with distinct anchors at opposite ends
/// (-30,0) and (30,0). Each anchor gets a small, mirrored cluster of 3 jobs with non-uniform
/// production values (2, 3, 1) so the `ProductionValue` metric is a genuine sum, not a disguised
/// job count. The two clusters are geometric mirror images, so on this fixture "already balanced"
/// and "already correctly territoried" coincide for every metric -- this is a convergence sanity
/// check, not a test that PUSH can *force* rebalancing (that's `problem_grid` below, via
/// production-value texture) or a build-time check.
fn problem_two_drivers_shared_start(metric: BalancePeriodMetric) -> TerritoryFixture {
    let start = (0., 0.);
    let anchor_coords = [(-30., 0.), (30., 0.)];
    let offsets = [(-2., 0.), (0., 3.), (2., -2.)];
    let values = [2., 3., 1.];

    let mut home_vehicle = HashMap::new();
    let mut job_value = HashMap::new();
    let mut jobs = Vec::new();
    let mut vehicles = Vec::new();
    let mut vehicle_ids = Vec::new();

    for (d, &anchor) in anchor_coords.iter().enumerate() {
        let driver_id = format!("driver{d}");
        let vehicle_id = format!("{driver_id}_1");
        vehicle_ids.push(vehicle_id.clone());

        vehicles.push(VehicleType {
            shifts: vec![create_default_vehicle_shift_with_locations(start, anchor)],
            ..create_vehicle_with_driver_id(&driver_id, vec![10], &driver_id)
        });

        for (j, (&(dx, dy), &value)) in offsets.iter().zip(values.iter()).enumerate() {
            let job_id = format!("{driver_id}-job{j}");
            home_vehicle.insert(job_id.clone(), vehicle_id.clone());
            job_value.insert(job_id.clone(), value);
            jobs.push(create_delivery_job_with_production_value(&job_id, (anchor.0 + dx, anchor.1 + dy), value));
        }
    }

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = (0..anchor_coords.len())
        .map(|d| (format!("driver{d}"), location_index(&plan, &fleet, anchor_coords[d])))
        .collect::<HashMap<_, _>>();

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective(TerritoryProximity::Distance, metric, anchors),
        MinimizeCost,
    ]);

    TerritoryFixture { problem: Problem { plan, fleet, objectives }, home_vehicle, job_value, vehicle_ids }
}

/// `cluster_sizes.len()` drivers sharing a start at the origin, anchors laid out on a circle
/// around it, each with `cluster_sizes[d]` jobs clustered tightly around its own anchor (jitter
/// well inside the gap between neighbouring anchors, so "nearest anchor" is unambiguous by
/// construction). Every job has production value 1, so the `ProductionValue` metric here reduces
/// to a job count.
///
/// `cluster_sizes` is deliberately uneven (not `num_jobs / num_drivers` each) so quota balancing
/// has real work to do: every driver has the same shift window, so quotas are equal
/// (`sum(cluster_sizes) / len` each) while the *raw*, cost-only nearest-anchor assignment would
/// give each driver exactly its own cluster's size -- e.g. a driver whose cluster is oversized
/// relative to the mean is genuinely over quota under pure proximity assignment, and the
/// balance/PUSH mechanism must move some of its jobs to an under-quota driver to correct it. This
/// is what distinguishes this fixture from `problem_two_drivers_shared_start`, whose mirrored
/// clusters are already balanced by construction and so cannot tell a working balance mechanism
/// from a no-op one.
fn problem_grid(cluster_sizes: &[usize], metric: BalancePeriodMetric) -> TerritoryFixture {
    let num_drivers = cluster_sizes.len();
    let start = (0., 0.);
    let radius = 50.;

    let mut home_vehicle = HashMap::new();
    let mut job_value = HashMap::new();
    let mut jobs = Vec::new();
    let mut vehicles = Vec::new();
    let mut vehicle_ids = Vec::new();
    let mut anchor_coords = Vec::new();

    for (d, &cluster_size) in cluster_sizes.iter().enumerate() {
        let theta = 2. * std::f64::consts::PI * d as Float / num_drivers as Float;
        let anchor = (radius * theta.cos(), radius * theta.sin());
        anchor_coords.push(anchor);

        let driver_id = format!("driver{d}");
        let vehicle_id = format!("{driver_id}_1");
        vehicle_ids.push(vehicle_id.clone());

        vehicles.push(VehicleType {
            shifts: vec![create_default_vehicle_shift_with_locations(start, anchor)],
            // Capacity is a generous, fixed headroom above the largest cluster (not tied to this
            // cluster's own size): the hard capacity constraint should never be what forces
            // balancing here, the soft territory/balance objective should.
            ..create_vehicle_with_driver_id(&driver_id, vec![40], &driver_id)
        });

        for j in 0..cluster_size {
            let dx = ((j % 4) as Float - 1.5) * 2.;
            let dy = ((j / 4) as Float - 1.5) * 2.;
            let job_id = format!("{driver_id}-job{j}");
            home_vehicle.insert(job_id.clone(), vehicle_id.clone());
            job_value.insert(job_id.clone(), 1.);
            jobs.push(create_delivery_job_with_production_value(&job_id, (anchor.0 + dx, anchor.1 + dy), 1.));
        }
    }

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = (0..num_drivers)
        .map(|d| (format!("driver{d}"), location_index(&plan, &fleet, anchor_coords[d])))
        .collect::<HashMap<_, _>>();

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective(TerritoryProximity::Distance, metric, anchors),
        MinimizeCost,
    ]);

    TerritoryFixture { problem: Problem { plan, fleet, objectives }, home_vehicle, job_value, vehicle_ids }
}

fn solve(problem: Problem, generations: usize) -> Solution {
    let matrix = create_matrix_from_problem(&problem);
    solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), generations)
}

// endregion

// region: solution-grading helpers

/// Job ids of a tour's real activities (i.e. excluding the synthetic `departure`/`arrival` stops).
fn tour_job_ids(tour: &Tour) -> impl Iterator<Item = &str> {
    tour.stops
        .iter()
        .flat_map(|stop| stop.activities().iter())
        .filter(|a| a.activity_type != "departure" && a.activity_type != "arrival")
        .map(|a| a.job_id.as_str())
}

/// Per-tour activity counts, indexed against the *full* vehicle roster (a driver with zero jobs --
/// no tour at all in the solution -- counts as 0, it is not silently omitted).
fn activity_counts_per_tour(solution: &Solution, vehicle_ids: &[String]) -> Vec<usize> {
    vehicle_ids
        .iter()
        .map(|vid| solution.tours.iter().find(|t| &t.vehicle_id == vid).map(|t| tour_job_ids(t).count()).unwrap_or(0))
        .collect()
}

/// Per-tour totals of `metric`, indexed against the full vehicle roster like
/// `activity_counts_per_tour`. `Distance`/`Duration` read the tour's own travel statistic
/// (matches what the `territory` feature bills a route for those metrics); `Activities` counts
/// real activities; `ProductionValue` sums each served job's value from `job_value`.
fn per_tour_metric_totals(
    solution: &Solution,
    vehicle_ids: &[String],
    metric: &BalancePeriodMetric,
    job_value: &HashMap<String, Float>,
) -> Vec<Float> {
    vehicle_ids
        .iter()
        .map(|vid| {
            let Some(tour) = solution.tours.iter().find(|t| &t.vehicle_id == vid) else { return 0. };
            match metric {
                BalancePeriodMetric::Distance => tour.statistic.distance as Float,
                BalancePeriodMetric::Duration => tour.statistic.duration as Float,
                BalancePeriodMetric::Activities => tour_job_ids(tour).count() as Float,
                BalancePeriodMetric::ProductionValue => {
                    tour_job_ids(tour).map(|id| job_value.get(id).copied().unwrap_or(0.)).sum()
                }
            }
        })
        .collect()
}

/// Population coefficient of variation (stddev / mean); 0 when the mean is ~0 (nothing to balance).
fn coefficient_of_variation(values: &[Float]) -> Float {
    let n = values.len() as Float;
    let mean = values.iter().sum::<Float>() / n;
    if mean.abs() < 1e-9 {
        return 0.;
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<Float>() / n;
    variance.sqrt() / mean
}

/// True when the per-tour totals for `metric` are balanced within a meaningful tolerance
/// (coefficient of variation below 0.2 -- e.g. a 2-way split is allowed to be at most roughly
/// 60/40, not free to be arbitrarily lopsided).
fn is_balanced_within_tolerance(
    solution: &Solution,
    vehicle_ids: &[String],
    metric: &BalancePeriodMetric,
    job_value: &HashMap<String, Float>,
) -> bool {
    coefficient_of_variation(&per_tour_metric_totals(solution, vehicle_ids, metric, job_value)) < 0.2
}

/// True when every driver's `metric` total is within `band` (a fraction, e.g. 0.25 == 25%) of the
/// mean/quota share. Unlike `is_balanced_within_tolerance` (a population-wide CV), this catches a
/// single outlier driver even when the rest of the fleet is fine.
fn each_driver_within_quota_band(
    solution: &Solution,
    vehicle_ids: &[String],
    metric: &BalancePeriodMetric,
    job_value: &HashMap<String, Float>,
    band: Float,
) -> bool {
    let totals = per_tour_metric_totals(solution, vehicle_ids, metric, job_value);
    let mean = totals.iter().sum::<Float>() / totals.len() as Float;
    if mean.abs() < 1e-9 {
        return true;
    }
    totals.iter().all(|&t| (t - mean).abs() / mean <= band)
}

/// Fraction of served jobs whose assigned vehicle is not their "home" vehicle (the anchor they
/// were clustered around by construction). 0.0 means every job stayed within its own territory.
fn territory_overlap_ratio(solution: &Solution, home_vehicle: &HashMap<String, String>) -> Float {
    let mut served = 0usize;
    let mut off_home = 0usize;
    for tour in &solution.tours {
        for job_id in tour_job_ids(tour) {
            served += 1;
            if home_vehicle.get(job_id).map(String::as_str) != Some(tour.vehicle_id.as_str()) {
                off_home += 1;
            }
        }
    }
    if served == 0 { 0. } else { off_home as Float / served as Float }
}

fn no_job_served_by_far_anchor(solution: &Solution, home_vehicle: &HashMap<String, String>) -> bool {
    territory_overlap_ratio(solution, home_vehicle) == 0.
}

// endregion

const SMALL_GENERATIONS: usize = 800;
const SCALE_GENERATIONS: usize = 1500;

#[test]
fn territory_forms_balanced_clusters_from_shared_start() {
    let fixture = problem_two_drivers_shared_start(BalancePeriodMetric::Activities);
    let solution = solve(fixture.problem.clone(), SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &fixture.vehicle_ids);
    let overlap = territory_overlap_ratio(&solution, &fixture.home_vehicle);
    eprintln!("=== territory_forms_balanced_clusters_from_shared_start ===");
    eprintln!("  activity counts per tour: {counts:?}");
    eprintln!("  territory overlap ratio: {overlap:.3}");

    assert!((counts[0] as i32 - counts[1] as i32).abs() <= 1, "unbalanced activity counts: {counts:?}");
    assert!(no_job_served_by_far_anchor(&solution, &fixture.home_vehicle), "territory overlap ratio was {overlap:.3}, expected 0");
}

#[test]
fn territory_balances_for_each_metric() {
    for metric in [
        BalancePeriodMetric::Distance,
        BalancePeriodMetric::Duration,
        BalancePeriodMetric::Activities,
        BalancePeriodMetric::ProductionValue,
    ] {
        let fixture = problem_two_drivers_shared_start(metric.clone());
        let solution = solve(fixture.problem.clone(), SMALL_GENERATIONS);

        assert!(solution.unassigned.is_none(), "metric {metric:?}: unexpected unassigned jobs: {:?}", solution.unassigned);

        let totals = per_tour_metric_totals(&solution, &fixture.vehicle_ids, &metric, &fixture.job_value);
        let cv = coefficient_of_variation(&totals);
        eprintln!("=== territory_balances_for_each_metric: {metric:?} ===");
        eprintln!("  per-tour totals: {totals:?}, cv: {cv:.3}");

        assert!(
            is_balanced_within_tolerance(&solution, &fixture.vehicle_ids, &metric, &fixture.job_value),
            "metric {metric:?} not balanced: totals={totals:?}, cv={cv:.3}"
        );
    }
}

#[test]
fn territory_scales_to_many_drivers() {
    // Deliberately uneven raw cluster sizes: two outlier clusters (19, 11) sit at +-26.7% off the
    // mean of 15 -- just outside the 25% quota band asserted below -- while the rest sit exactly
    // on the mean. A working balance/PUSH mechanism only needs to move a handful of jobs between
    // the two outliers to land back inside the band; a disabled/reverted one would leave the raw
    // (19, 11) split in place and fail the band assertion outright.
    let cluster_sizes = [19, 15, 15, 15, 15, 15, 15, 11];
    assert_eq!(cluster_sizes.iter().sum::<usize>(), 120);
    let fixture = problem_grid(&cluster_sizes, BalancePeriodMetric::ProductionValue);
    let solution = solve(fixture.problem.clone(), SCALE_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let totals = per_tour_metric_totals(&solution, &fixture.vehicle_ids, &BalancePeriodMetric::ProductionValue, &fixture.job_value);
    let overlap = territory_overlap_ratio(&solution, &fixture.home_vehicle);
    eprintln!("=== territory_scales_to_many_drivers ===");
    eprintln!("  per-tour production-value totals: {totals:?}");
    eprintln!("  territory overlap ratio: {overlap:.3}");

    assert!(
        each_driver_within_quota_band(&solution, &fixture.vehicle_ids, &BalancePeriodMetric::ProductionValue, &fixture.job_value, 0.25),
        "a driver was >25% off its quota share: totals={totals:?}"
    );
    assert!(overlap < 0.1, "territory overlap ratio too high: {overlap:.3}");
}
