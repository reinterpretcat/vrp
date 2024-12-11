use super::*;
use crate::construction::heuristics::ActivityContext;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::{ActivityBuilder, RouteContextBuilder};

fn tier(tier: usize) -> usize {
    tier
}

fn loc(idx: usize) -> Location {
    idx
}

fn simple(id: usize) -> LocationDetail {
    LocationDetail::Simple(id)
}

fn compound(ids: &[usize]) -> LocationDetail {
    LocationDetail::Compound(ids.into_iter().copied().collect())
}

enum Estimate {
    Tier(usize),
    Penalty,
    DoublePenalty,
}

parameterized_test! {can_estimate_activity, test_data, {
    let mut index = HierarchyIndex::new(3);
    let (data, expected) = test_data;
    assert_eq!(data.len(), 3);

    for (location, loc_data) in data.into_iter().enumerate() {
        for (tier, detail) in loc_data.into_iter() {
            index.insert(location, tier, detail).expect("cannot insert test data");
        }
    }
    let insertion_idx = 0;

    let expected = match expected {
        Estimate::Tier(tier) => index.tiers.get_value(tier).expect("cannot get tier value"),
        Estimate::Penalty => index.tiers.penalty_value(),
        Estimate::DoublePenalty => 2 * index.tiers.penalty_value(),
    };

    can_estimate_activity_impl(index, insertion_idx, expected);
}}

can_estimate_activity! {
    case01_tier_zero_all_three_with_same_tier: (vec![
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
    ], Estimate::Tier(0)),

    case02_tier_zero_new_cluster_split: (vec![
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(2))],
    ], Estimate::DoublePenalty),

    case03_tier_zero_same: (vec![
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
    ], Estimate::Tier(0)),

    case04_tier_one_new_with_different_id: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(3))],
      vec![(tier(0), simple(1)), (tier(1), simple(3))],
      vec![(tier(0), simple(2)), (tier(1), simple(3))],
    ], Estimate::Tier(1)),

    case05_tier_one_share_same_id: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(1))],
      vec![(tier(0), simple(1)), (tier(1), simple(1))],
      vec![(tier(0), simple(2)), (tier(1), simple(1))],
    ], Estimate::Tier(1)),

    case06_tier_zero_new_as_border_cluster: (vec![
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(2))],
      vec![(tier(0), simple(2))],
    ], Estimate::Tier(0)),

    case07_tier_one_new_as_border_cluster: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(4))],
      vec![(tier(0), simple(2)), (tier(1), simple(5))],
      vec![(tier(0), simple(3)), (tier(1), simple(4))],
    ], Estimate::Tier(1)),

    case08_tier_one_new_as_border_cluster: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(4))],
      vec![(tier(0), simple(2)), (tier(1), simple(5))],
      vec![(tier(0), simple(3)), (tier(1), simple(5))],
    ], Estimate::Tier(1)),

    case09_tier_two_new_all_different: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(4)), (tier(2), simple(7))],
      vec![(tier(0), simple(2)), (tier(1), simple(5)), (tier(2), simple(8))],
      vec![(tier(0), simple(3)), (tier(1), simple(6)), (tier(2), simple(9))],
    ], Estimate::Penalty),

    case09_tier_two_new_split_cluster: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(0), simple(2)), (tier(1), simple(4)), (tier(2), simple(6))],
    ], Estimate::DoublePenalty),

    case10_tier_two_new_join_cluster_on_third_tier: (vec![
      vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(0), simple(2)), (tier(1), simple(4)), (tier(2), simple(5))],
    ], Estimate::Tier(2)),

    case11_tier_two_new_join_cluster_with_skipping_middle_tier: (vec![
      vec![(tier(0), simple(1)), (tier(2), simple(3))],
      vec![(tier(0), simple(1)), (tier(2), simple(3))],
      vec![(tier(0), simple(2)), (tier(2), simple(3))],
    ], Estimate::Tier(2)),

    case12_tier_two_new_join_cluster_skipping_middle_tier: (vec![
      vec![(tier(0), simple(1)), (tier(2), simple(3))],
      vec![(tier(0), simple(1)), (tier(2), simple(3))],
      vec![(tier(0), simple(2)), (tier(2), simple(4))],
    ], Estimate::DoublePenalty),

    case13_tier_two_new_join_cluster_skipping_zero_tier: (vec![
      vec![(tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(1), simple(3)), (tier(2), simple(5))],
      vec![(tier(1), simple(4)), (tier(2), simple(5))],
    ], Estimate::Tier(2)),

    case14_all_empty: (vec![
      vec![],
      vec![],
      vec![],
    ], Estimate::Penalty),

    case15_new_empty: (vec![
      vec![(tier(0), simple(1))],
      vec![(tier(0), simple(1))],
      vec![],
    ], Estimate::DoublePenalty),

    case16_old_empty: (vec![
      vec![(tier(0), simple(1))],
      vec![],
      vec![],
    ], Estimate::Penalty),

    case16_compound_intersection: (vec![
      vec![(tier(0), compound(&[1, 2]))],
      vec![(tier(0), compound(&[2, 3]))],
      vec![(tier(0), compound(&[2]))],
    ], Estimate::Tier(0)),

    case17_compound_disjoint: (vec![
      vec![(tier(0), compound(&[1, 2]))],
      vec![(tier(0), compound(&[3, 4]))],
      vec![(tier(0), compound(&[5, 6]))],
    ], Estimate::Penalty),
}

fn can_estimate_activity_impl(hierarchy_index: HierarchyIndex, insertion_idx: usize, expected: usize) {
    let objective = HierarchicalAreasObjective { mode: HierarchicalAreaMode::OnlyLocal, hierarchy_index };
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;
    let route_ctx = RouteContextBuilder::default().build();
    let next_activity = ActivityBuilder::with_location(1).build();
    let activity_ctx = ActivityContext {
        index: insertion_idx,
        prev: &ActivityBuilder::with_location(0).build(),
        target: &ActivityBuilder::with_location(2).build(),
        next: Some(&next_activity),
    };

    let result = objective.estimate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx));

    assert_eq!(result, expected as f64);
}

#[test]
fn can_generate_expected_tier_values() {
    assert_eq!(vec![0, 1, 3], Tiers::new(3).iter().map(|tier| tier.value()).collect::<Vec<_>>());

    assert_eq!(vec![0, 1, 3, 7, 15], Tiers::new(5).iter().map(|tier| tier.value()).collect::<Vec<_>>());

    assert_eq!(vec![0, 1, 3, 7, 15, 31, 63], Tiers::new(7).iter().map(|tier| tier.value()).collect::<Vec<_>>());
}
