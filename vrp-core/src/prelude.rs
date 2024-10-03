//! This module reimports commonly used types.

// Reimport core types

pub use crate::construction::{
    features::{CapacityFeatureBuilder, MinimizeUnassignedBuilder, TransportFeatureBuilder},
    heuristics::{InsertionContext, MoveContext, RouteContext, RouteState, SolutionContext, SolutionState},
};
pub use crate::solver::{Solver, VrpConfigBuilder};
pub use crate::{
    custom_activity_state, custom_dimension, custom_extra_property, custom_solution_state, custom_tour_state,
};

pub use crate::models::{
    common::{Cost, Demand, Dimensions, SingleDimLoad},
    problem::{
        ActivityCost, Fleet, Job, Jobs, MultiBuilder, SimpleTransportCost, SingleBuilder, TransportCost, Vehicle,
        VehicleBuilder, VehicleDetailBuilder,
    },
    {ConstraintViolation, Feature, FeatureBuilder, FeatureConstraint, FeatureObjective, FeatureState, ViolationCode},
    {Extras, GoalContext, GoalContextBuilder, Problem, ProblemBuilder, Solution},
};

// Reimport rosomaxa types
pub use rosomaxa::{
    evolution::EvolutionConfigBuilder,
    utils::{DefaultRandom, Environment, Float, GenericError, GenericResult, InfoLogger, Random},
};
