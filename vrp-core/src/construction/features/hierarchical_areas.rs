// TODO remove allow macros

#![allow(dead_code)]
#![allow(unused_variables)]

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/hierarchical_areas_test.rs"]
mod hierarchical_areas_test;

use crate::construction::enablers::FeatureCombinator;
use crate::construction::heuristics::ActivityContext;
use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Creates a feature to guide search considering hierarchy of areas.
/// A `cost_feature` used to calculate the cost of transition which will be considered as a base for
/// an internal penalty.
pub fn create_hierarchical_areas_feature(
    cost_feature: Feature,
    hierarchy_index: Arc<HierarchyIndex>,
) -> GenericResult<Feature> {
    if cost_feature.objective.is_none() {
        return Err(GenericError::from("hierarchical areas requires cost feature to have an objective"));
    }

    // use feature combinator to properly interpret additional constraints and states.
    FeatureCombinator::default()
        .use_name(cost_feature.name.as_str())
        .add_feature(cost_feature)
        .set_objective_combinator(move |objectives| {
            if objectives.len() != 1 {
                return Err(GenericError::from("hierarchical areas feature requires exactly one cost objective"));
            }

            let (_, objective) = objectives[0].clone();
            let hierarchy_index = hierarchy_index.clone();

            Ok(Some(Arc::new(HierarchicalAreasObjective { objective, hierarchy_index })))
        })
        .combine()
}

/// Represents a hierarchical index of areas at different level of details.
pub struct HierarchyIndex {
    tiers: Tiers,
    index: HashMap<Location, HashMap<Tier, LocationDetail>>,
}

impl HierarchyIndex {
    fn new(levels: usize) -> Self {
        Self { tiers: Tiers::new(levels), index: HashMap::new() }
    }

    fn insert(&mut self, location: Location, level: usize, detail: LocationDetail) -> GenericResult<()> {
        if level + 1 > self.tiers.0.len() {
            return Err(From::from(format!(
                "wrong tier level: {level} when total levels is {}",
                self.tiers.0.len().saturating_sub(1)
            )));
        }

        let Some(value) = self.tiers.get_value(level) else {
            return Err(From::from(format!("cannot get tier for level={level}")));
        };

        self.index.entry(location).or_default().insert(Tier(value), detail);

        Ok(())
    }

    fn get(&self, location: &Location) -> Option<&HashMap<Tier, LocationDetail>> {
        self.index.get(location)
    }
}

/// Represents a tier in hierarchy of areas.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Tier(usize);

impl Tier {
    fn value(&self) -> usize {
        self.0
    }
}

/// Keeps track of possible tiers in hierarchy.
#[derive(Debug)]
struct Tiers(Vec<Tier>, usize);

impl Tiers {
    fn new(levels: usize) -> Self {
        let values = (0..levels)
            .scan(0, |value, idx| {
                *value = if idx == 0 { 0 } else { *value * 2 + 1 };

                Some(Tier(*value))
            })
            .collect::<Vec<_>>();

        let max_value = values.last().map(|tier| tier.0).unwrap_or_default();

        Self(values, max_value * 2 + 1)
    }

    /// Iterates through all tiers starting from the lowest one.
    fn iter(&self) -> impl Iterator<Item = &Tier> {
        self.0.iter()
    }

    /// Gets value for the tier at a specific level.
    fn get_value(&self, level: usize) -> Option<usize> {
        Some(self.0.get(level)?.value())
    }

    /// Returns a penalty value which is outside any tier values.
    fn penalty_value(&self) -> usize {
        self.1
    }
}

/// Represents specific detail for location.
pub enum LocationDetail {
    /// Unique attribute. Different locations will be checked for its equality.
    Simple(usize),
    /// Multiple attributes. Different locations will be checked whether there is an intersection between their sets.
    Compound(HashSet<usize>),
}

impl LocationDetail {
    /// Creates a new `LocationDetails::Simple`.
    pub fn new_simple(value: usize) -> Self {
        Self::Simple(value)
    }

    /// Creates a new `LocationDetails::Compound`.
    pub fn new_compound(values: HashSet<usize>) -> Self {
        Self::Compound(values)
    }

    /// Returns the value of `LocationDetails::Simple`.
    pub fn as_simple(&self) -> Option<&usize> {
        match self {
            Self::Simple(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value of `LocationDetails::Compound`.
    pub fn as_compound(&self) -> Option<&HashSet<usize>> {
        match self {
            Self::Compound(values) => Some(values),
            _ => None,
        }
    }
}

struct HierarchicalAreasObjective {
    objective: Arc<dyn FeatureObjective>,
    hierarchy_index: Arc<HierarchyIndex>,
}

impl FeatureObjective for HierarchicalAreasObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        self.objective.fitness(insertion_ctx)
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let activity_ctx = match move_ctx {
            MoveContext::Route { .. } => return Cost::default(),
            MoveContext::Activity { activity_ctx, .. } => activity_ctx,
        };

        get_penalty(activity_ctx, &self.hierarchy_index) as Cost
    }
}

fn get_penalty(activity_ctx: &ActivityContext, hierarchy_index: &HierarchyIndex) -> usize {
    let estimate_fn = |from, to| estimate_leg_cost(from, to, hierarchy_index);

    let prev = activity_ctx.prev.place.location;
    let target = activity_ctx.target.place.location;
    let next = activity_ctx.next.map(|next| next.place.location);

    if let Some(next) = next {
        let prev_target = estimate_fn(prev, target);
        let target_next = estimate_fn(target, next);

        prev_target.min(target_next)
    } else {
        estimate_fn(prev, target)
    }
}

fn estimate_leg_cost(from: Location, to: Location, hierarchy_index: &HierarchyIndex) -> usize {
    hierarchy_index
        .get(&from)
        .zip(hierarchy_index.get(&to))
        .iter()
        .flat_map(|from_to| hierarchy_index.tiers.iter().map(move |tier| (from_to, tier)))
        .filter_map(|((from, to), tier)| {
            from.get(tier).zip(to.get(tier)).and_then(|(left, right)| match (left, right) {
                (LocationDetail::Simple(left), LocationDetail::Simple(right)) if left == right => Some(tier.value()),
                (LocationDetail::Compound(left), LocationDetail::Compound(right)) if !left.is_disjoint(right) => {
                    Some(tier.value())
                }
                (LocationDetail::Simple(simple), LocationDetail::Compound(compound))
                | (LocationDetail::Compound(compound), LocationDetail::Simple(simple))
                    if compound.contains(simple) =>
                {
                    Some(tier.value())
                }
                _ => None,
            })
        })
        // stop at the first match as we're starting from the lowest tier
        .next()
        .unwrap_or_else(|| hierarchy_index.tiers.penalty_value())
}
