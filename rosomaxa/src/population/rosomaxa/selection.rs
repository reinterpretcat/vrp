use super::{IndividualNetwork, RosomaxaContext, RosomaxaSolution};
use crate::HeuristicStatistics;
use crate::algorithms::gsom::{Coordinate, Input};
use crate::algorithms::math::relative_distance;
use crate::evolution::objectives::HeuristicObjective;
use crate::population::HeuristicPopulation;
use crate::population::elitism::Alternative;
use crate::utils::{Float, Random};
use rand::prelude::SliceRandom;
use std::cmp::Ordering;
use std::f64::consts::E;

// Three generations revisit promising GSOM regions, while the fourth samples the whole occupied map uniformly.
const BASIN_SELECTION_PERIOD: usize = 4;

// Reserve roughly one quarter of the GSOM parent budget for uniformly sampled occupied nodes.
const BASIN_COVERAGE_DIVISOR: usize = 4;

/// Keeps a GSOM coordinate and its cached distance to the closest selected basin.
pub(super) struct BasinCandidate {
    pub(super) coordinate: Coordinate,
    min_distance: Float,
}

impl BasinCandidate {
    /// Creates a candidate whose nearest-selected distance will be initialized before ranking.
    pub(super) fn new(coordinate: Coordinate) -> Self {
        Self { coordinate, min_distance: Float::MAX }
    }
}

/// Groups the selected basin representatives with the complete set of detected sinks.
struct BasinSelection<'a> {
    representatives: &'a [BasinCandidate],
    sink_coordinates: &'a [Coordinate],
}

impl BasinSelection<'_> {
    /// Selects a good non-minimum node far from the basin representatives as a cheap approximation of a basin shoulder.
    fn select_shoulder<C, O, S>(
        &self,
        network: &IndividualNetwork<C, O, S>,
        occupied_coordinates: &[Coordinate],
        objective: &O,
    ) -> Option<Coordinate>
    where
        C: RosomaxaContext<Solution = S>,
        O: HeuristicObjective<Solution = S> + Alternative,
        S: RosomaxaSolution<Context = C>,
    {
        if self.representatives.len() < 2 {
            return None;
        }

        let mut candidates = occupied_coordinates
            .iter()
            .filter(|coordinate| self.sink_coordinates.binary_search(coordinate).is_err())
            .copied()
            .collect::<Vec<_>>();
        let get_node =
            |coordinate: &Coordinate| network.find(coordinate).expect("selection candidates belong to the GSOM");
        let get_solution = |coordinate: &Coordinate| {
            get_node(coordinate).storage.regular.best().expect("selection candidates have a solution")
        };
        let compare =
            |left: &Coordinate, right: &Coordinate| objective.total_order(get_solution(left), get_solution(right));

        let candidate_size = get_basin_candidate_size(candidates.len(), 1);
        if candidate_size < candidates.len() {
            candidates.select_nth_unstable_by(candidate_size, compare);
            candidates.truncate(candidate_size);
        }

        let selected_weights = self
            .representatives
            .iter()
            .map(|candidate| get_node(&candidate.coordinate).weights.as_slice())
            .collect::<Vec<_>>();
        candidates
            .into_iter()
            .map(|coordinate| {
                let weights = get_node(&coordinate).weights.as_slice();
                let min_distance = selected_weights
                    .iter()
                    .map(|selected| network.squared_distance(weights, selected))
                    .min_by(Float::total_cmp)
                    .unwrap_or_default();

                (coordinate, min_distance)
            })
            .max_by(|(left, left_distance), (right, right_distance)| {
                left_distance.total_cmp(right_distance).then_with(|| compare(right, left))
            })
            .map(|(coordinate, _)| coordinate)
    }
}

pub(super) fn prepare_selection<C, O, S>(
    network: &IndividualNetwork<C, O, S>,
    selection_coordinates: &mut Vec<Coordinate>,
    basin_candidates: &mut Vec<BasinCandidate>,
    random: &dyn Random,
    objective: &O,
    statistics: &HeuristicStatistics,
    selection_size: usize,
) where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    selection_coordinates.clear();
    selection_coordinates.extend(
        network.iter().filter_map(
            |(coordinate, node)| {
                if node.storage.regular.size() > 0 { Some(*coordinate) } else { None }
            },
        ),
    );

    selection_coordinates.shuffle(&mut random.get_rng());

    // Periodic full-map batches keep exploration coherent instead of diluting every basin-focused batch.
    if !is_basin_selection_generation(statistics.generation) {
        return;
    }

    let node_selection_budget = selection_size.saturating_sub(get_min_elite_selection_size(selection_size));
    // Most node searches revisit distinct basins; a smaller share keeps every occupied region reachable.
    let coverage_selection_size = get_coverage_selection_size(node_selection_budget);
    let basin_selection_size = node_selection_budget.saturating_sub(coverage_selection_size);
    if basin_selection_size == 0 {
        return;
    }

    basin_candidates.clear();
    let should_select_shoulder =
        is_basin_shoulder_selection_generation(statistics.generation, statistics.improvement_1000_ratio);
    // Local sinks are cheap basin proxies: tracing every occupied node to its sink would add more map scans here.
    basin_candidates.extend(
        selection_coordinates
            .iter()
            .filter(|coordinate| is_basin_sink_node(network, coordinate, objective))
            .map(|coordinate| BasinCandidate::new(*coordinate)),
    );
    // Preserve the complete sink set before quality gating and diversity ordering mutate the candidates.
    let sink_coordinates = if should_select_shoulder {
        let mut coordinates = basin_candidates.iter().map(|candidate| candidate.coordinate).collect::<Vec<_>>();
        coordinates.sort_unstable();
        coordinates
    } else {
        Vec::new()
    };
    let compare_quality = |left: &BasinCandidate, right: &BasinCandidate| {
        let left = network
            .find(&left.coordinate)
            .and_then(|node| node.storage.regular.best())
            .expect("basin candidates are occupied GSOM nodes");
        let right = network
            .find(&right.coordinate)
            .and_then(|node| node.storage.regular.best())
            .expect("basin candidates are occupied GSOM nodes");

        objective.total_order(left, right)
    };

    // Keep a rank-based quality gate independent of the objective's numeric scale. Small basin sets remain intact.
    let candidate_size = get_basin_candidate_size(basin_candidates.len(), basin_selection_size);
    if candidate_size < basin_candidates.len() {
        basin_candidates.select_nth_unstable_by(candidate_size, compare_quality);
        basin_candidates.truncate(candidate_size);
    }

    let selected_size = basin_selection_size.min(basin_candidates.len());
    if selected_size == 0 {
        return;
    }

    // Rotate the reference within the quality-gated set, then add candidates far from everything already selected.
    let reference_idx = random.uniform_int(0, basin_candidates.len() as i32 - 1) as usize;
    select_diverse_basin_candidates(basin_candidates, selected_size, reference_idx, |left, right| {
        let left = network.find(left).expect("basin candidates belong to the GSOM");
        let right = network.find(right).expect("basin candidates belong to the GSOM");
        network.squared_distance(left.weights.as_slice(), right.weights.as_slice())
    });

    // Occasionally replace the least distinctive basin with a good distant slope. Put it first so extra elite
    // selections cannot truncate the escape attempt from the GSOM prefix.
    let basins =
        BasinSelection { representatives: &basin_candidates[..selected_size], sink_coordinates: &sink_coordinates };
    if should_select_shoulder && let Some(shoulder) = basins.select_shoulder(network, selection_coordinates, objective)
    {
        basin_candidates[..selected_size].rotate_right(1);
        basin_candidates[0] = BasinCandidate::new(shoulder);
    }

    // Interleave basin representatives with shuffled nodes, preserving a direct path to every occupied region.
    promote_basin_coordinates(
        selection_coordinates,
        &basin_candidates[..selected_size],
        node_selection_budget,
        coverage_selection_size,
    );
}

/// Checks whether the best solution in a GSOM node represents a map-local basin sink.
fn is_basin_sink_node<C, O, S>(network: &IndividualNetwork<C, O, S>, coordinate: &Coordinate, objective: &O) -> bool
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    let Some(node) = network.find(coordinate) else { return false };
    let Some(solution) = node.storage.regular.best() else { return false };
    let Coordinate(x, y) = *coordinate;

    let neighbors = [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
        .into_iter()
        .filter_map(|neighbor_coordinate| {
            let neighbor = network.find(&neighbor_coordinate)?;
            let neighbor_solution = neighbor.storage.regular.best()?;
            Some((neighbor_coordinate, objective.total_order(solution, neighbor_solution)))
        });

    is_basin_sink(coordinate, neighbors)
}

pub(super) fn select_initial_data<S, O>(data: Vec<S>, objective: &O, keep_size: usize) -> Vec<S>
where
    S: Input,
    O: HeuristicObjective<Solution = S>,
{
    select_diverse_data(data, keep_size, |left, right| objective.total_order(left, right))
}

pub(super) fn select_diverse_data<S>(mut data: Vec<S>, keep_size: usize, compare: impl Fn(&S, &S) -> Ordering) -> Vec<S>
where
    S: Input,
{
    if data.len() <= keep_size {
        return data;
    }

    data.sort_by(&compare);

    // Remove the weakest quarter before looking at feature distance. Otherwise, an incomplete or otherwise poor
    // solution can be retained simply because it is far away from every useful solution.
    let candidate_size = data.len().saturating_mul(3).div_ceil(4).max(keep_size).min(data.len());
    data.truncate(candidate_size);

    let mut selected = Vec::with_capacity(keep_size);
    selected.push(data.swap_remove(0));

    while selected.len() < keep_size && !data.is_empty() {
        let candidate_idx = data
            .iter()
            .map(|candidate| {
                selected
                    .iter()
                    .map(|known| relative_distance(candidate.weights().iter(), known.weights().iter()))
                    .min_by(Float::total_cmp)
                    .expect("at least one solution is selected")
            })
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .expect("at least one solution remains");

        selected.push(data.swap_remove(candidate_idx));
    }

    selected
}

/// Moves selected basin representatives into the parent prefix while preserving uniformly sampled coverage slots.
pub(super) fn promote_basin_coordinates(
    coordinates: &mut [Coordinate],
    basins: &[BasinCandidate],
    selection_size: usize,
    coverage_size: usize,
) {
    let mut basin_idx = 0;
    let mut coverage_idx = 0;
    let selection_size = selection_size.min(coordinates.len());

    for selection_idx in 0..selection_size {
        // Place coverage at the midpoint of each equally sized part of the selected prefix.
        let is_coverage_slot = coverage_idx < coverage_size
            && selection_idx == (2 * coverage_idx + 1) * selection_size / (2 * coverage_size);

        if is_coverage_slot {
            let remaining_basins = &basins[basin_idx..];
            if remaining_basins.iter().any(|candidate| candidate.coordinate == coordinates[selection_idx])
                && let Some(candidate_idx) = ((selection_idx + 1)..coordinates.len()).find(|&candidate_idx| {
                    remaining_basins.iter().all(|candidate| candidate.coordinate != coordinates[candidate_idx])
                })
            {
                coordinates.swap(selection_idx, candidate_idx);
            }
            coverage_idx += 1;
        } else if let Some(candidate) = basins.get(basin_idx)
            && let Some(candidate_idx) =
                (selection_idx..coordinates.len()).find(|&idx| coordinates[idx] == candidate.coordinate)
        {
            coordinates.swap(selection_idx, candidate_idx);
            basin_idx += 1;
        }
    }
}

/// Orders the selected prefix using incremental maximum--minimum distance from the already selected representatives.
pub(super) fn select_diverse_basin_candidates(
    candidates: &mut [BasinCandidate],
    selected_size: usize,
    reference_idx: usize,
    distance: impl Fn(&Coordinate, &Coordinate) -> Float,
) {
    let selected_size = selected_size.min(candidates.len());
    if selected_size == 0 {
        return;
    }

    candidates.swap(0, reference_idx);
    if selected_size == 1 {
        return;
    }

    let reference = candidates[0].coordinate;
    for candidate in &mut candidates[1..] {
        candidate.min_distance = distance(&candidate.coordinate, &reference);
    }

    for selected_idx in 1..selected_size {
        let candidate_idx = (selected_idx..candidates.len())
            .max_by(|&left, &right| candidates[left].min_distance.total_cmp(&candidates[right].min_distance))
            .expect("at least one basin candidate remains");

        candidates.swap(selected_idx, candidate_idx);

        if selected_idx + 1 < selected_size {
            let selected = candidates[selected_idx].coordinate;
            for candidate in &mut candidates[(selected_idx + 1)..] {
                let candidate_distance = distance(&candidate.coordinate, &selected);
                if candidate_distance.total_cmp(&candidate.min_distance).is_lt() {
                    candidate.min_distance = candidate_distance;
                }
            }
        }
    }
}

/// Returns true when no cardinal neighbor dominates the coordinate, resolving equal-fitness plateaus by coordinate.
pub(super) fn is_basin_sink(
    coordinate: &Coordinate,
    mut neighbors: impl Iterator<Item = (Coordinate, Ordering)>,
) -> bool {
    // Coordinate order gives equal-fitness plateaus a stable direction instead of selecting every plateau node.
    neighbors.all(|(neighbor, order)| order == Ordering::Less || (order == Ordering::Equal && *coordinate < neighbor))
}

/// Gets the elite share of the exploration budget. Each block of four parents adds one elite selection tournament,
/// with more elite searches used when the population is not improving.
pub(super) fn get_elite_selection_size(
    selection_size: usize,
    improvement_ratio: Float,
    mut is_hit: impl FnMut(Float) -> bool,
) -> usize {
    if selection_size <= 6 {
        return get_min_elite_selection_size(selection_size);
    }

    let probability = (1. - 1. / (1. + E.powf(-10. * (improvement_ratio - 0.166)))) as Float;
    let tournament_count = selection_size.div_ceil(4);

    (1..=tournament_count)
        .map(|idx| if is_hit(probability / idx as Float) { 2 } else { 1 })
        .sum::<usize>()
        .min(selection_size - 1)
}

/// Returns the elite budget guaranteed before randomized extra elite tournaments are considered.
pub(super) fn get_min_elite_selection_size(selection_size: usize) -> usize {
    if selection_size <= 6 { selection_size.min(1) } else { selection_size.div_ceil(4) }
}

/// Keeps the better half of basin sinks, or enough candidates to fill all basin positions.
pub(super) fn get_basin_candidate_size(basin_count: usize, selection_size: usize) -> usize {
    basin_count.div_ceil(2).max(selection_size).min(basin_count)
}

/// Uses basin-focused ordering for three generations and uniform map ordering for the fourth.
pub(super) fn is_basin_selection_generation(generation: usize) -> bool {
    !generation.is_multiple_of(BASIN_SELECTION_PERIOD)
}

/// Tests a basin shoulder occasionally while search is progressing and every basin cycle when it is nearly stagnant.
pub(super) fn is_basin_shoulder_selection_generation(generation: usize, improvement_ratio: Float) -> bool {
    let period =
        if improvement_ratio <= 0.1 { BASIN_SELECTION_PERIOD } else { BASIN_SELECTION_PERIOD * BASIN_SELECTION_PERIOD };

    generation % period == 1
}

/// Reserves roughly one quarter of node positions for uniformly shuffled map coverage.
pub(super) fn get_coverage_selection_size(selection_size: usize) -> usize {
    if selection_size <= 1 {
        return 0;
    }

    (selection_size / BASIN_COVERAGE_DIVISOR).max(1)
}

/// Cools direct node-alternative sampling as the map matures. Retained alternatives still participate in replay.
pub(super) fn get_node_alternative_probability(termination_estimate: Float) -> Float {
    const INITIAL_PROBABILITY: Float = 0.05;

    INITIAL_PROBABILITY * (1. - termination_estimate.clamp(0., 1.))
}

/// Gets the exploitation budget by using half of the configured selection capacity.
pub(super) fn get_exploitation_selection_size(selection_size: usize) -> usize {
    selection_size.div_ceil(2).max(2)
}
