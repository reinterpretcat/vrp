//! This module reimports commonly used types.

// Reimport core types
pub use crate::solver::{Solver, VrpConfigBuilder};

pub use crate::construction::features::{CapacityFeatureBuilder, MinimizeUnassignedBuilder, TransportFeatureBuilder};

pub use crate::models::{
    common::{Demand, SingleDimLoad},
    problem::{
        ActivityCost, Fleet, Jobs, SimpleTransportCost, SingleBuilder, TransportCost, VehicleBuilder,
        VehicleDetailBuilder,
    },
    {GoalContext, GoalContextBuilder, Problem, ProblemBuilder, Solution},
};

// Reimport rosomaxa types
pub use rosomaxa::{
    evolution::EvolutionConfigBuilder,
    utils::{compare_floats, DefaultRandom, Environment, GenericError, GenericResult, InfoLogger, Random},
};
