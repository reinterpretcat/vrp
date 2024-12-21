// TODO remove allow macros

#![allow(dead_code)]
#![allow(unused_variables)]

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/hierarchical_areas_test.rs"]
mod hierarchical_areas_test;

use crate::construction::enablers::FeatureCombinator;
use crate::construction::heuristics::ActivityContext;
use crate::models::common::Profile;
use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A type which stores a list of clusters at different levels of hierarchy starting from the lowest one.
pub type ClusterHierarchy = Vec<HashMap<Location, Vec<Location>>>;

/// Creates a feature to guide search considering hierarchy of areas.
/// A `cost_feature` used to calculate the cost of transition which will be considered as a base for
/// an internal penalty.
pub fn create_hierarchical_areas_feature<F>(
    cost_feature: Feature,
    clusters: &ClusterHierarchy,
    distance_fn: F,
) -> GenericResult<Feature>
where
    F: Fn(&Profile, Location, Location) -> Cost + Send + Sync + 'static,
{
    if cost_feature.objective.is_none() {
        return Err(GenericError::from("hierarchical areas requires cost feature to have an objective"));
    }

    let hierarchy_index = Arc::new(HierarchyIndex::try_from(clusters)?);

    let hierarchy_feature = FeatureBuilder::default()
        .with_name(cost_feature.name.as_str()) // name will be ignored
        .with_state(HierarchicalAreasState { hierarchy_index: hierarchy_index.clone() })
        .build()?;

    // use feature combinator to properly interpret additional constraints and states.
    FeatureCombinator::default()
        .use_name(cost_feature.name.as_str())
        .add_features(&[cost_feature, hierarchy_feature])
        .set_objective_combinator(move |objectives| {
            if objectives.len() != 1 {
                return Err(GenericError::from("hierarchical areas feature requires exactly one cost objective"));
            }

            let objective = objectives[0].1.clone();

            Ok(Some(Arc::new(HierarchicalAreasObjective { objective, distance_fn, hierarchy_index })))
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

    /// Gets tier at the given level.
    fn get(&self, level: usize) -> Option<&Tier> {
        self.0.get(level)
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
    fn max_penalty_value(&self) -> usize {
        self.1
    }
}

/// Represents specific detail for location.
#[derive(Debug, Eq, PartialEq)]
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

custom_tour_state!(MedoidIndex typeof HashMap<Tier, HashSet<Location>>);

struct HierarchicalAreasObjective<F> {
    objective: Arc<dyn FeatureObjective>,
    distance_fn: F,
    hierarchy_index: Arc<HierarchyIndex>,
}

impl<F> FeatureObjective for HierarchicalAreasObjective<F>
where
    F: Fn(&Profile, Location, Location) -> Cost + Send + Sync,
{
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        // use inner objective estimation for global fitness here
        // `estimate` function is supposed to guide search in a more efficient way
        self.objective.fitness(insertion_ctx)
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => self.get_job_cost(solution_ctx, route_ctx, job),
            MoveContext::Activity { activity_ctx, .. } => self.get_activity_cost(move_ctx, activity_ctx),
        }
    }
}

impl<F> HierarchicalAreasObjective<F>
where
    F: Fn(&Profile, Location, Location) -> Cost + Send + Sync,
{
    fn get_job_cost(&self, solution_ctx: &SolutionContext, route_ctx: &RouteContext, job: &Job) -> Cost {
        let max_tier_penalty = self.hierarchy_index.tiers.max_penalty_value();
        let profile = &route_ctx.route().actor.vehicle.profile;

        job.places()
            .filter_map(|place| place.location.as_ref())
            .filter_map(|location| self.hierarchy_index.get(location).map(|cluster| (location, cluster)))
            .flat_map(|(location, cluster)| {
                self.hierarchy_index
                    .tiers
                    .iter()
                    .filter_map(|tier| cluster.get(tier).map(|detail| (tier, detail)))
                    .filter_map(|(tier, detail)| detail.as_simple().copied().map(|medoid| (tier, medoid)))
                    .map(|(tier, medoid)| {
                        // find out whether this medoid is already seen in any other route
                        // if so, calculate overlap factor using tier value
                        solution_ctx
                            .routes
                            .iter()
                            .filter(|&other| route_ctx != other)
                            .filter_map(|route_ctx| route_ctx.state().get_medoid_index())
                            .filter_map(|medoid_index| medoid_index.get(tier))
                            .filter_map(|other_medoids| other_medoids.get(&medoid))
                            // more penalty on more fine-grained tiers
                            .map(|_| (max_tier_penalty - tier.value()) as Float / max_tier_penalty as Float)
                            .next()
                            .map(|overlap_factor| overlap_factor * (self.distance_fn)(profile, *location, medoid))
                            .unwrap_or_default()
                    })
            })
            .sum::<Cost>()
    }

    fn get_activity_cost(&self, move_ctx: &MoveContext<'_>, activity_ctx: &ActivityContext) -> Cost {
        // estimate penalty based on hierarchy
        let penalty = get_activity_penalty(activity_ctx, &self.hierarchy_index) as Cost;

        // normalize penalty to be in range [0, 1]
        let ratio = penalty / self.hierarchy_index.tiers.max_penalty_value() as Cost;

        // get base cost as inner objective estimate
        let base_cost = self.objective.estimate(move_ctx);

        // return cost in range [base_cost, 2*base_cost]
        base_cost * ratio + base_cost
    }
}

fn get_activity_penalty(activity_ctx: &ActivityContext, hierarchy_index: &HierarchyIndex) -> usize {
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
        .unwrap_or_else(|| hierarchy_index.tiers.max_penalty_value())
}

struct HierarchicalAreasState {
    hierarchy_index: Arc<HierarchyIndex>,
}

impl FeatureState for HierarchicalAreasState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_idx: usize, job: &Job) {
        // that should not happen
        let Some(route_ctx) = solution_ctx.routes.get_mut(route_idx) else {
            return;
        };

        // no index: create a new one from all activities
        let Some(medoid_index) = route_ctx.state().get_medoid_index() else {
            self.accept_route_state(route_ctx);
            return;
        };

        // iterate over all place locations, get respective tier's medoids, and update medoid index
        // NOTE this approach is suboptimal for jobs with alternative locations, but it's fine for now
        let medoid_index = job
            .places()
            .filter_map(|place| place.location.as_ref())
            .filter_map(|location| self.hierarchy_index.get(location))
            .flat_map(|cluster| {
                self.hierarchy_index
                    .tiers
                    .iter()
                    .filter_map(|tier| cluster.get(tier).map(|detail| (tier, detail)))
                    .filter_map(|(tier, detail)| detail.as_simple().copied().map(|medoid| (tier, medoid)))
            })
            .fold(medoid_index.clone(), |mut acc, (tier, medoid)| {
                acc.entry(tier.clone()).or_default().insert(medoid);
                acc
            });

        route_ctx.state_mut().set_medoid_index(medoid_index);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        // iterate over all activities, get respective tier's medoids, and create medoid index
        let medoid_index = route_ctx
            .route()
            .tour
            .all_activities()
            .filter(|activity| activity.job.is_some())
            .map(|activity| activity.place.location)
            .filter_map(|location| self.hierarchy_index.get(&location))
            .flat_map(|index| index.iter())
            // TODO so far, ignore compound variant
            .filter_map(|(tier, detail)| detail.as_simple().map(|&medoid| (tier.clone(), medoid)))
            .fold(HashMap::<Tier, HashSet<Location>>::new(), |mut acc, (tier, medoid)| {
                acc.entry(tier).or_default().insert(medoid);
                acc
            });

        route_ctx.state_mut().set_medoid_index(medoid_index);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx
            .routes
            .iter_mut()
            .filter(|route_ctx| route_ctx.is_stale())
            .for_each(|route_ctx| self.accept_route_state(route_ctx));
    }
}

/// Conversion logic from k-medoids clustering algorithm result.
/// We assume sorting from the lowest level to the highest one.
impl TryFrom<&ClusterHierarchy> for HierarchyIndex {
    type Error = GenericError;

    fn try_from(hierarchy: &ClusterHierarchy) -> Result<Self, Self::Error> {
        if hierarchy.first().map_or(true, |clusters| clusters.len() != 2) {
            return Err(From::from("a first level of hierarchy should have 2 clusters"));
        }

        let levels = hierarchy.len();
        let mut index = HierarchyIndex::new(levels);

        // reverse hierarchy to start from the level with the fewest clusters
        hierarchy.iter().rev().enumerate().try_for_each(|(level, clusters)| {
            // use medoid location as a key in location detail
            clusters.iter().try_for_each(|(medoid, cluster)| {
                cluster
                    .iter()
                    .try_for_each(|&location| index.insert(location, level, LocationDetail::new_simple(*medoid)))
            })
        })?;

        Ok(index)
    }
}
