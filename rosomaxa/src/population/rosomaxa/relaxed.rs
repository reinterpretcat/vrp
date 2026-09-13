use super::selection::{BasinCandidate, select_diverse_basin_candidates, select_diverse_data};
use super::storage::{compare_relaxed, is_relaxed_selectable};
use super::{IndividualNetwork, RosomaxaContext, RosomaxaSolution};
use crate::algorithms::gsom::Coordinate;
use crate::algorithms::math::relative_distance;
use crate::evolution::objectives::HeuristicObjective;
use crate::population::elitism::Alternative;
use std::collections::HashSet;

pub(super) fn select_relaxed_solution<'a, C, O, S>(
    network: &'a IndividualNetwork<C, O, S>,
    objective: &O,
    reference: &S,
    archive_size: usize,
    prefer_continuation: bool,
) -> Option<&'a S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    let Coordinate(x, y) = network.find_bmu_coordinate(reference);
    let coordinate = Coordinate(x, y);

    let local =
        network.find(&coordinate).and_then(|node| node.storage.relaxed.select(prefer_continuation)).or_else(|| {
            [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
                .iter()
                .filter_map(|coordinate| {
                    network.find(coordinate).and_then(|node| node.storage.relaxed.select(prefer_continuation))
                })
                .min_by(|left, right| compare_relaxed(objective, left, right))
        });
    if local.is_some() {
        return local;
    }

    let mut candidates = network
        .iter_nodes()
        .filter_map(|node| node.storage.relaxed.select(prefer_continuation).map(|solution| (node, solution)))
        .collect::<Vec<_>>();

    // Seed an uncovered regular region while the archive has capacity. Once full, continue from its global
    // representatives instead of repeatedly restarting relaxed search from every sparse GSOM region.
    if candidates.len() < archive_size {
        return None;
    }

    candidates.sort_unstable_by(|(_, left), (_, right)| compare_relaxed(objective, left, right));
    candidates.truncate(candidates.len().div_ceil(2));
    candidates
        .into_iter()
        .max_by(|(left, _), (right, _)| {
            network
                .squared_distance(left.weights.as_slice(), reference.weights())
                .total_cmp(&network.squared_distance(right.weights.as_slice(), reference.weights()))
        })
        .map(|(_, solution)| solution)
}

pub(super) fn select_relaxed_archive<'a, O, S>(
    archive: &'a [S],
    objective: &O,
    reference: &S,
    prefer_continuation: bool,
) -> Option<&'a S>
where
    O: HeuristicObjective<Solution = S>,
    S: RosomaxaSolution,
{
    let mut candidates = archive
        .iter()
        .filter(|solution| {
            is_relaxed_selectable(*solution) && (!prefer_continuation || solution.is_relaxed_continuation())
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|left, right| compare_relaxed(objective, left, right));
    candidates.truncate(candidates.len().div_ceil(2).max(1));

    candidates.into_iter().max_by(|left, right| {
        relative_distance(left.weights().iter(), reference.weights().iter())
            .total_cmp(&relative_distance(right.weights().iter(), reference.weights().iter()))
    })
}

pub(super) fn prune_relaxed_archive<O, S>(archive: &mut Vec<S>, objective: &O, keep_size: usize)
where
    O: HeuristicObjective<Solution = S>,
    S: RosomaxaSolution,
{
    // Historical checkpoints can help describe a node while its episode is active there, but a flat archive has
    // no such locality. Keep its limited capacity for work that selection can actually resume.
    archive.retain(is_relaxed_selectable);
    archive.sort_unstable_by(|left, right| compare_relaxed(objective, left, right));
    archive.dedup_by(|left, right| left.is_same(right));

    if archive.len() > keep_size {
        *archive = select_diverse_data(std::mem::take(archive), keep_size, |left, right| {
            compare_relaxed(objective, left, right)
        });
    }
}

pub(super) fn prune_relaxed<C, O, S>(network: &mut IndividualNetwork<C, O, S>, objective: &O, keep_size: usize)
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    let mut candidates = network
        .iter()
        .filter_map(|(coordinate, node)| node.storage.relaxed.select(false).map(|solution| (*coordinate, solution)))
        .collect::<Vec<_>>();
    if candidates.len() <= keep_size {
        return;
    }

    candidates.sort_unstable_by(|(_, left), (_, right)| compare_relaxed(objective, left, right));

    // Apply the same quality gate as initial GSOM seeding, then retain distant structural representatives.
    let candidate_size = candidates.len().saturating_mul(3).div_ceil(4).max(keep_size).min(candidates.len());
    candidates.truncate(candidate_size);
    let mut candidates =
        candidates.into_iter().map(|(coordinate, _)| BasinCandidate::new(coordinate)).collect::<Vec<_>>();
    select_diverse_basin_candidates(&mut candidates, keep_size, 0, |left, right| {
        let left = network.find(left).expect("relaxed candidate belongs to the GSOM");
        let right = network.find(right).expect("relaxed candidate belongs to the GSOM");
        network.squared_distance(left.weights.as_slice(), right.weights.as_slice())
    });

    let selected = candidates[..keep_size].iter().map(|candidate| candidate.coordinate).collect::<HashSet<_>>();
    network.iter_nodes_mut().for_each(|node| {
        if !selected.contains(&node.coordinate) {
            node.storage.relaxed.clear();
        }
    });
}
