use crate::construction::features::{create_capacity_limit_feature, create_minimize_transport_costs_feature};
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use crate::models::common::{Demand, SingleDimLoad};
use crate::models::{Feature, Goal, GoalContext};

pub fn create_simple_demand(size: i32) -> Demand<SingleDimLoad> {
    if size > 0 {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::new(size), SingleDimLoad::default()),
            delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
        }
    } else {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
            delivery: (SingleDimLoad::new(-size), SingleDimLoad::default()),
        }
    }
}

pub fn create_simple_dynamic_demand(size: i32) -> Demand<SingleDimLoad> {
    if size > 0 {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::new(size)),
            delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
        }
    } else {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
            delivery: (SingleDimLoad::default(), SingleDimLoad::new(-size)),
        }
    }
}

pub fn create_goal_ctx_with_features(features: Vec<Feature>, feature_map: Vec<Vec<&str>>) -> GoalContext {
    let feature_map: Vec<Vec<String>> =
        feature_map.iter().map(|names| names.iter().map(|name| name.to_string()).collect()).collect();

    let goal = Goal::no_alternatives(feature_map.clone(), feature_map);

    GoalContext::new(features.as_slice(), goal).unwrap()
}

pub fn create_goal_ctx_with_feature(feature: Feature) -> GoalContext {
    create_goal_ctx_with_features(
        vec![feature.clone()],
        if feature.objective.is_some() { vec![vec![feature.name.as_str()]] } else { vec![] },
    )
}

pub fn create_goal_ctx_with_transport() -> GoalContext {
    create_minimize_transport_costs_feature(
        "transport",
        TestTransportCost::new_shared(),
        TestActivityCost::new_shared(),
        1,
    )
    .map(create_goal_ctx_with_feature)
    .unwrap()
}

pub fn create_goal_ctx_with_simple_capacity() -> GoalContext {
    create_capacity_limit_feature::<SingleDimLoad>("capacity", 2).map(create_goal_ctx_with_feature).unwrap()
}
