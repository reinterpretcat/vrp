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
/// "belongs" to by construction (its nearest anchor), the production value it was given, its own
/// location (needed to recompute the feature's own `Distance`/`Duration` accounting -- see
/// `nearest_anchor_prox` below), the full anchor location set (ditto), and the full vehicle id
/// roster (so a driver that ends up completely unused -- zero jobs, no tour in the solution at all
/// -- is still counted as a zero, not silently dropped from the balance stats).
struct TerritoryFixture {
    problem: Problem,
    home_vehicle: HashMap<String, String>,
    job_value: HashMap<String, Float>,
    job_location: HashMap<String, (f64, f64)>,
    anchor_locations: Vec<(f64, f64)>,
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

fn territory_objective(
    proximity: TerritoryProximity,
    balance: BalancePeriodMetric,
    anchors: HashMap<String, Vec<usize>>,
) -> Objective {
    territory_objective_with_quota(proximity, balance, anchors, None)
}

/// Like [`territory_objective`], but with a caller-supplied per-driver `quota` map -- the input that
/// replaces the solver's own capacity-share derivation. `None` selects the derive path.
fn territory_objective_with_quota(
    proximity: TerritoryProximity,
    balance: BalancePeriodMetric,
    anchors: HashMap<String, Vec<usize>>,
    quota: Option<HashMap<String, Float>>,
) -> Objective {
    territory_objective_with_quota_and_weights(proximity, balance, anchors, quota, None)
}

/// Like [`territory_objective_with_quota`], but also with a caller-supplied per-driver power
/// `weights` map. `None` leaves every weight at `0.0`, which is what supplied anchors always got.
fn territory_objective_with_quota_and_weights(
    proximity: TerritoryProximity,
    balance: BalancePeriodMetric,
    anchors: HashMap<String, Vec<usize>>,
    quota: Option<HashMap<String, Float>>,
    weights: Option<HashMap<String, Float>>,
) -> Objective {
    // Zero deadband: these E2E assertions regress the strict, exact-balance PULL/PUSH mechanism.
    Territory {
        proximity,
        balance: Some(balance),
        balance_tolerance: 0.0,
        anchors,
        weights,
        allow_idle_drivers: false,
        quota,
    }
}

/// Two drivers sharing a start location at the origin, with distinct anchors at opposite ends
/// (-30,0) and (30,0). The two clusters are deliberately **uneven**: driver0 gets 2 jobs, driver1
/// gets 4, with non-uniform production values on both sides (so `ProductionValue` is a genuine
/// sum, not a disguised job count) -- while both drivers share the same capacity and shift time
/// window, so their balance quotas come out *equal* (total metric / 2) regardless of which side
/// has more raw jobs. A cost-only, nearest-anchor assignment (i.e. what you get with the
/// `territory` objective removed, or with a no-op PUSH) leaves each driver with its own cluster's
/// raw total: `[2, 4]` jobs, `[5, 10]` production value, `[5, 11]` distance/duration proximity --
/// imbalanced on *every* `BalancePeriodMetric` (confirmed by ablation, see the task report). Only
/// a working PUSH, relocating driver1's boundary job (the one closest to driver0's anchor) over to
/// driver0, brings the solution back within the balance tolerance asserted below. This is what
/// distinguishes this fixture from a mirrored, already-balanced-by-construction one -- see
/// `problem_grid`'s doc comment for the larger-scale version of the same idea.
fn problem_two_drivers_shared_start(metric: BalancePeriodMetric) -> TerritoryFixture {
    let start = (0., 0.);
    let anchor_coords = [(-30., 0.), (30., 0.)];
    // (offset-from-anchor, production value) per job, per driver. driver0's cluster is smaller (2
    // jobs) than driver1's (4 jobs); per-job values are drawn from the same 1-3 scale on both
    // sides so the imbalance comes from genuine cluster composition, not a rigged value multiplier.
    let clusters: [&[((f64, f64), Float)]; 2] = [
        &[((-2., 0.), 2.), ((0., 3.), 3.)],
        &[((-2., 0.), 3.), ((0., 3.), 3.), ((2., -2.), 2.), ((3., 1.), 2.)],
    ];

    let mut home_vehicle = HashMap::new();
    let mut job_value = HashMap::new();
    let mut job_location = HashMap::new();
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

        for (j, &((dx, dy), value)) in clusters[d].iter().enumerate() {
            let job_id = format!("{driver_id}-job{j}");
            let loc = (anchor.0 + dx, anchor.1 + dy);
            home_vehicle.insert(job_id.clone(), vehicle_id.clone());
            job_value.insert(job_id.clone(), value);
            job_location.insert(job_id.clone(), loc);
            jobs.push(create_delivery_job_with_production_value(&job_id, loc, value));
        }
    }

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = (0..anchor_coords.len())
        .map(|d| (format!("driver{d}"), vec![location_index(&plan, &fleet, anchor_coords[d])]))
        .collect::<HashMap<_, _>>();

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective(TerritoryProximity::Distance, metric, anchors),
        MinimizeCost,
    ]);

    TerritoryFixture {
        problem: Problem { plan, fleet, objectives },
        home_vehicle,
        job_value,
        job_location,
        anchor_locations: anchor_coords.to_vec(),
        vehicle_ids,
    }
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
/// is the larger-scale (many drivers, larger clusters) version of the same uneven-cluster idea
/// `problem_two_drivers_shared_start` uses at 2-driver scale.
fn problem_grid(cluster_sizes: &[usize], metric: BalancePeriodMetric) -> TerritoryFixture {
    let num_drivers = cluster_sizes.len();
    let start = (0., 0.);
    let radius = 50.;

    let mut home_vehicle = HashMap::new();
    let mut job_value = HashMap::new();
    let mut job_location = HashMap::new();
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
            let loc = (anchor.0 + dx, anchor.1 + dy);
            home_vehicle.insert(job_id.clone(), vehicle_id.clone());
            job_value.insert(job_id.clone(), 1.);
            job_location.insert(job_id.clone(), loc);
            jobs.push(create_delivery_job_with_production_value(&job_id, loc, 1.));
        }
    }

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = (0..num_drivers)
        .map(|d| (format!("driver{d}"), vec![location_index(&plan, &fleet, anchor_coords[d])]))
        .collect::<HashMap<_, _>>();

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective(TerritoryProximity::Distance, metric, anchors),
        MinimizeCost,
    ]);

    TerritoryFixture {
        problem: Problem { plan, fleet, objectives },
        home_vehicle,
        job_value,
        job_location,
        anchor_locations: anchor_coords,
        vehicle_ids,
    }
}

/// Two drivers anchored at (-30,0) and (30,0) sharing a start at the origin, with six jobs placed on
/// the perpendicular bisector `x = 0`. Every job is then *exactly* equidistant from both anchors, so
/// PULL is identically zero for any assignment and the territory objective reduces to its PUSH term
/// -- which is driven by nothing but the quotas. The split the solver settles on therefore IS the
/// quota split, and that is what lets this fixture tell a supplied quota from a derived one (the
/// clustered fixtures above cannot: there, geography and quota push the same way). Both drivers get
/// the same shift window, so the derived quota is an even 3/3.
///
/// Returns the problem plus the full vehicle id roster, so a driver that ends up with no tour at all
/// still counts as a zero rather than being silently dropped.
fn problem_equidistant_jobs(quota: Option<HashMap<String, Float>>) -> (Problem, Vec<String>) {
    problem_equidistant_jobs_with_weights(quota, None)
}

/// Like [`problem_equidistant_jobs`], but also carrying a caller-supplied per-driver power `weights`
/// map. This fixture is where a weight is legible on its own: PULL is zero for every assignment
/// while the weights are absent, so any split away from the even 3/3 is the weights and nothing
/// else. With `w_d1 = w`, a job served by driver0 reaches `prox - (prox - w) = w` of PULL and the
/// same job on driver1 reaches zero, while pulling one job across costs `1 x 60` of PUSH (the
/// anchor-to-anchor ground distance) -- so a weight well above 60 per job makes driver1 the home for
/// all six.
fn problem_equidistant_jobs_with_weights(
    quota: Option<HashMap<String, Float>>,
    weights: Option<HashMap<String, Float>>,
) -> (Problem, Vec<String>) {
    let start = (0., 0.);
    let anchor_coords = [(-30., 0.), (30., 0.)];
    let job_offsets = [-5., -3., -1., 1., 3., 5.];

    let mut vehicles = Vec::new();
    let mut vehicle_ids = Vec::new();
    for (d, &anchor) in anchor_coords.iter().enumerate() {
        let driver_id = format!("driver{d}");
        vehicle_ids.push(format!("{driver_id}_1"));
        vehicles.push(VehicleType {
            shifts: vec![create_default_vehicle_shift_with_locations(start, anchor)],
            ..create_vehicle_with_driver_id(&driver_id, vec![10], &driver_id)
        });
    }

    let jobs = job_offsets
        .iter()
        .enumerate()
        .map(|(j, &y)| create_delivery_job_with_production_value(&format!("job{j}"), (0., y), 1.))
        .collect();

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = (0..anchor_coords.len())
        .map(|d| (format!("driver{d}"), vec![location_index(&plan, &fleet, anchor_coords[d])]))
        .collect::<HashMap<_, _>>();

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective_with_quota_and_weights(
            TerritoryProximity::Distance,
            BalancePeriodMetric::Activities,
            anchors,
            quota,
            weights,
        ),
        MinimizeCost,
    ]);

    (Problem { plan, fleet, objectives }, vehicle_ids)
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

/// Proximity from a job location to the nearest of `anchor_locations`, computed the same way
/// `create_matrix_from_problem` builds the routing matrix (squared-coordinate-difference Euclidean
/// distance, rounded to the matrix's integer resolution). This mirrors
/// `TerritoryShared::nearest_anchor_prox` in `vrp-core/src/construction/features/territory.rs`
/// against the *full* anchor set (not just the serving vehicle's own anchor) -- which is exactly
/// what `job_metric`'s `Distance`/`Duration` branch bills per job, independent of which vehicle
/// ends up serving it.
fn nearest_anchor_prox(job_loc: (f64, f64), anchor_locations: &[(f64, f64)]) -> Float {
    anchor_locations
        .iter()
        .map(|&(ax, ay)| ((job_loc.0 - ax).powf(2.) + (job_loc.1 - ay).powf(2.)).sqrt().round())
        .fold(Float::INFINITY, |a, b| a.min(b))
}

/// Per-tour totals of `metric`, indexed against the full vehicle roster like
/// `activity_counts_per_tour`. `Distance`/`Duration` sum, over the tour's jobs, each job's
/// proximity to its own *globally* nearest anchor (via `nearest_anchor_prox`) -- this is what the
/// `territory` feature's `job_metric` actually bills for those metrics (see
/// `TerritoryShared::job_metric` / `nearest_anchor_prox`), which is independent of the serving
/// vehicle and therefore *not* the same as the tour's own driven-travel statistic. `Activities`
/// counts real activities; `ProductionValue` sums each served job's value from `job_value`.
fn per_tour_metric_totals(
    solution: &Solution,
    vehicle_ids: &[String],
    metric: &BalancePeriodMetric,
    job_value: &HashMap<String, Float>,
    job_location: &HashMap<String, (f64, f64)>,
    anchor_locations: &[(f64, f64)],
) -> Vec<Float> {
    vehicle_ids
        .iter()
        .map(|vid| {
            let Some(tour) = solution.tours.iter().find(|t| &t.vehicle_id == vid) else { return 0. };
            match metric {
                BalancePeriodMetric::Distance | BalancePeriodMetric::Duration => tour_job_ids(tour)
                    .filter_map(|id| job_location.get(id))
                    .map(|&loc| nearest_anchor_prox(loc, anchor_locations))
                    .sum(),
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
    job_location: &HashMap<String, (f64, f64)>,
    anchor_locations: &[(f64, f64)],
) -> bool {
    coefficient_of_variation(&per_tour_metric_totals(solution, vehicle_ids, metric, job_value, job_location, anchor_locations)) < 0.2
}

/// True when every driver's `metric` total is within `band` (a fraction, e.g. 0.25 == 25%) of the
/// mean/quota share. Unlike `is_balanced_within_tolerance` (a population-wide CV), this catches a
/// single outlier driver even when the rest of the fleet is fine.
fn each_driver_within_quota_band(
    solution: &Solution,
    vehicle_ids: &[String],
    metric: &BalancePeriodMetric,
    job_value: &HashMap<String, Float>,
    job_location: &HashMap<String, (f64, f64)>,
    anchor_locations: &[(f64, f64)],
    band: Float,
) -> bool {
    let totals = per_tour_metric_totals(solution, vehicle_ids, metric, job_value, job_location, anchor_locations);
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

// endregion

const SMALL_GENERATIONS: usize = 800;
const SCALE_GENERATIONS: usize = 1500;

/// Overlap bound for the small (6-job, 2-driver) fixture. Its uneven `[2, 4]` raw split needs
/// exactly one boundary job to cross for balance (1/6 ~= 0.167); this bound allows that single
/// necessary move plus a little slack, while still ruling out excessive churn (e.g. two jobs
/// crossing, 2/6 ~= 0.333, or a wholesale swap) that would mean PUSH overcorrected rather than
/// making the minimal fix.
const SMALL_OVERLAP_BOUND: Float = 0.3;

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
    assert!(
        overlap < SMALL_OVERLAP_BOUND,
        "territory overlap ratio was {overlap:.3}, expected < {SMALL_OVERLAP_BOUND}"
    );
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

        let totals = per_tour_metric_totals(
            &solution,
            &fixture.vehicle_ids,
            &metric,
            &fixture.job_value,
            &fixture.job_location,
            &fixture.anchor_locations,
        );
        let cv = coefficient_of_variation(&totals);
        eprintln!("=== territory_balances_for_each_metric: {metric:?} ===");
        eprintln!("  per-tour totals: {totals:?}, cv: {cv:.3}");

        assert!(
            is_balanced_within_tolerance(
                &solution,
                &fixture.vehicle_ids,
                &metric,
                &fixture.job_value,
                &fixture.job_location,
                &fixture.anchor_locations
            ),
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

    let totals = per_tour_metric_totals(
        &solution,
        &fixture.vehicle_ids,
        &BalancePeriodMetric::ProductionValue,
        &fixture.job_value,
        &fixture.job_location,
        &fixture.anchor_locations,
    );
    let overlap = territory_overlap_ratio(&solution, &fixture.home_vehicle);
    eprintln!("=== territory_scales_to_many_drivers ===");
    eprintln!("  per-tour production-value totals: {totals:?}");
    eprintln!("  territory overlap ratio: {overlap:.3}");

    assert!(
        each_driver_within_quota_band(
            &solution,
            &fixture.vehicle_ids,
            &BalancePeriodMetric::ProductionValue,
            &fixture.job_value,
            &fixture.job_location,
            &fixture.anchor_locations,
            0.25
        ),
        "a driver was >25% off its quota share: totals={totals:?}"
    );
    assert!(overlap < 0.1, "territory overlap ratio too high: {overlap:.3}");
}

/// The supplied quota is used verbatim: "driver0" is quoted at 1 against a capacity share of 3 and
/// ends up carrying strictly less work than "driver1". On this fixture PULL is zero by construction
/// (see `problem_equidistant_jobs`), so nothing but the quota could have produced the split -- and a
/// build that ignored the input would land on the derived, even 3/3 asserted by the control test
/// below, failing the `<= 2` bound here.
#[test]
fn supplied_quota_shifts_work_away_from_the_low_quota_driver() {
    let quota = HashMap::from([("driver0".to_string(), 1.), ("driver1".to_string(), 5.)]);
    let (problem, vehicle_ids) = problem_equidistant_jobs(Some(quota));
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== supplied_quota_shifts_work_away_from_the_low_quota_driver ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    assert!(counts[0] < counts[1], "the low-quota driver must carry less work: {counts:?}");
    assert!(counts[0] <= 2, "the low-quota driver must land below its 3/3 capacity share: {counts:?}");
}

/// Supplied anchors keep the caller's power weights instead of flooring every `w_d` at zero, which
/// is what made a supplied territory strictly weaker than a derived one. On this fixture PULL is
/// zero by construction while the weights are absent, so nothing but the weights could move the
/// split: a build that dropped them would land on the even 3/3 of the control test below.
#[test]
fn supplied_weights_pull_work_toward_the_heavier_driver() {
    // 200 per job against the 60 of PUSH it costs to pull one across: driver1's power cell swallows
    // the whole bisector.
    let weights = HashMap::from([("driver0".to_string(), 0.), ("driver1".to_string(), 200.)]);
    let (problem, vehicle_ids) = problem_equidistant_jobs_with_weights(None, Some(weights));
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== supplied_weights_pull_work_toward_the_heavier_driver ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    assert!(counts[0] < counts[1], "the weighted driver must carry more work: {counts:?}");
    assert!(counts[0] <= 1, "the weighted cell must swallow the bisector, not settle on 3/3: {counts:?}");
}

/// The two edge cases `weights` inherits from `quota`, in one solve: a key matching no driver is
/// ignored, and a driver absent from a non-empty map keeps weight `0.0` rather than being an error.
/// "driver0" is left out entirely and a "ghost" key is added, yet the split matches the full-map
/// test above -- so the absent driver was read as zero and the unknown key perturbed nothing.
#[test]
fn supplied_weights_ignore_unknown_keys_and_default_absent_drivers_to_zero() {
    let weights = HashMap::from([("driver1".to_string(), 200.), ("ghost-driver".to_string(), 999.)]);
    let (problem, vehicle_ids) = problem_equidistant_jobs_with_weights(None, Some(weights));
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== supplied_weights_ignore_unknown_keys_and_default_absent_drivers_to_zero ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    assert!(counts[0] <= 1, "an absent driver keeps weight 0 and an unknown key is inert: {counts:?}");
}

/// Control for the test above: the SAME fixture with the weights omitted leaves every `w_d` at zero,
/// which restores the even split -- so the assertion above is reading the weights, not the geometry.
#[test]
fn omitted_weights_leave_every_power_weight_at_zero() {
    let (problem, vehicle_ids) = problem_equidistant_jobs_with_weights(None, None);
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== omitted_weights_leave_every_power_weight_at_zero ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    assert!((counts[0] as i32 - counts[1] as i32).abs() <= 1, "zero weights must split evenly: {counts:?}");
}

/// Control for the test above: the SAME fixture with the quota omitted derives it from total demand
/// and capacity share as before, which is an even 3/3 here.
#[test]
fn omitted_quota_derives_the_capacity_share() {
    let (problem, vehicle_ids) = problem_equidistant_jobs(None);
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== omitted_quota_derives_the_capacity_share ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    // Six jobs over two drivers makes the difference even, so "within 1" is exactly the 3/3 split.
    assert!((counts[0] as i32 - counts[1] as i32).abs() <= 1, "derived quota must split evenly: {counts:?}");
}

/// A quota key matching no driver is ignored rather than fatal: the solve completes and lands on the
/// same split the two real keys ask for.
#[test]
fn unknown_driver_key_in_supplied_quota_does_not_break_the_solve() {
    let quota =
        HashMap::from([("ghost-driver".to_string(), 99.), ("driver0".to_string(), 1.), ("driver1".to_string(), 5.)]);
    let (problem, vehicle_ids) = problem_equidistant_jobs(Some(quota));
    let solution = solve(problem, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let counts = activity_counts_per_tour(&solution, &vehicle_ids);
    eprintln!("=== unknown_driver_key_in_supplied_quota_does_not_break_the_solve ===");
    eprintln!("  activity counts per tour: {counts:?}");

    assert_eq!(counts.iter().sum::<usize>(), 6, "every job must be served: {counts:?}");
    assert!(counts[0] <= 2, "the unknown key must not perturb the two that match: {counts:?}");
}

/// Which vehicle served each job in the solution.
fn serving_vehicle(solution: &Solution) -> HashMap<String, String> {
    solution
        .tours
        .iter()
        .flat_map(|tour| tour_job_ids(tour).map(move |id| (id.to_string(), tour.vehicle_id.clone())))
        .collect()
}

/// End-to-end proof that a driver's WHOLE anchor list crosses the wire format and reaches the
/// objective, not just its first entry. "driver0" holds two far-apart patches, at (-40,0) and
/// (40,0); "driver1" holds one at (0,60). Two jobs sit beside each patch. Quotas are supplied large
/// enough that no driver can ever be over quota, so PUSH is identically zero and PULL alone decides
/// the assignment -- which makes this a direct read of the anchor geometry.
///
/// With both of "driver0"'s anchors honoured, every outer job sits in one of its own patches (PULL
/// 0) and belongs to it. If only one anchor of the pair counted, the two jobs beside the *other*
/// patch would sit 80 from "driver0" against 72 from "driver1", so PULL would hand them to
/// "driver1" -- and, since the territory objective outranks `MinimizeCost`, it would do so however
/// long the detour. Asserting both outer clusters land on "driver0" therefore fails on any
/// first-anchor-only reading, whichever of the two the map happened to yield first.
#[test]
fn a_driver_keeps_the_ground_around_every_anchor_in_its_list() {
    // Anchors have to be locations the routing matrix indexes, so each patch centre is itself a job.
    let job_specs = [
        ("j_left_a", (-40., 0.)),
        ("j_left_b", (-38., 0.)),
        ("j_right_a", (40., 0.)),
        ("j_right_b", (38., 0.)),
        ("j_up_a", (0., 60.)),
        ("j_up_b", (0., 58.)),
    ];
    let depot = (0., 0.);

    let jobs = job_specs.iter().map(|(id, loc)| create_delivery_job_with_production_value(id, *loc, 1.)).collect();
    let vehicles = ["driver0", "driver1"]
        .iter()
        .map(|driver_id| VehicleType {
            shifts: vec![create_default_vehicle_shift_with_locations(depot, depot)],
            ..create_vehicle_with_driver_id(driver_id, vec![10], driver_id)
        })
        .collect();

    let plan = Plan { jobs, ..create_empty_plan() };
    let fleet = Fleet { vehicles, ..create_default_fleet() };

    let anchors = HashMap::from([
        (
            "driver0".to_string(),
            vec![location_index(&plan, &fleet, (-40., 0.)), location_index(&plan, &fleet, (40., 0.))],
        ),
        ("driver1".to_string(), vec![location_index(&plan, &fleet, (0., 60.))]),
    ]);
    // Quotas far above any reachable load: nobody is ever a surplus, so PUSH stays 0 throughout and
    // the split is decided by PULL (i.e. by the anchors) alone.
    let quota = HashMap::from([("driver0".to_string(), 999.), ("driver1".to_string(), 999.)]);

    let objectives = Some(vec![
        MinimizeUnassigned { breaks: None },
        territory_objective_with_quota(
            TerritoryProximity::Distance,
            BalancePeriodMetric::Activities,
            anchors,
            Some(quota),
        ),
        MinimizeCost,
    ]);

    let solution = solve(Problem { plan, fleet, objectives }, SMALL_GENERATIONS);

    assert!(solution.unassigned.is_none(), "unexpected unassigned jobs: {:?}", solution.unassigned);

    let served_by = serving_vehicle(&solution);
    eprintln!("=== a_driver_keeps_the_ground_around_every_anchor_in_its_list ===");
    eprintln!("  job -> vehicle: {served_by:?}");

    for job_id in ["j_left_a", "j_left_b", "j_right_a", "j_right_b"] {
        assert_eq!(
            served_by.get(job_id).map(String::as_str),
            Some("driver0_1"),
            "{job_id} sits beside one of driver0's own anchors: {served_by:?}"
        );
    }
    for job_id in ["j_up_a", "j_up_b"] {
        assert_eq!(
            served_by.get(job_id).map(String::as_str),
            Some("driver1_1"),
            "{job_id} sits in driver1's patch: {served_by:?}"
        );
    }
}

// Manual perf benchmark for the territory objective's construction cost (the hot path that, at
// fleet scale, was eating the whole solve budget building the initial solution):
//   cargo test -p vrp-pragmatic --release bench_territory_construction -- --ignored --nocapture
#[test]
#[ignore]
fn bench_territory_construction() {
    let drivers = 60usize;
    let per = 10usize;
    let total_jobs = drivers * per;
    let fixture = problem_grid(&vec![per; drivers], BalancePeriodMetric::Activities);
    let matrix = create_matrix_from_problem(&fixture.problem);

    let start = std::time::Instant::now();
    let solution = solve_with_cheapest_insertion(fixture.problem.clone(), Some(vec![matrix]));
    let elapsed = start.elapsed();

    let unassigned = solution.unassigned.as_ref().map_or(0, |u| u.len());
    eprintln!(
        "BENCH territory construction: {drivers} drivers, {total_jobs} jobs -> cheapest-insertion {elapsed:?}, unassigned {unassigned}/{total_jobs}"
    );
}
