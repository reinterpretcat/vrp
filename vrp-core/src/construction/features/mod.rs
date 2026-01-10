//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::*;
use rosomaxa::prelude::*;
use std::sync::Arc;

mod breaks;
pub use self::breaks::*;

mod capacity;
pub(crate) use self::capacity::MaxVehicleLoadTourState;
pub use self::capacity::{CapacityFeatureBuilder, JobDemandDimension, VehicleCapacityDimension};

mod compatibility;
pub use self::compatibility::{JobCompatibilityDimension, create_compatibility_feature};

mod fast_service;
pub use self::fast_service::FastServiceFeatureBuilder;

mod fleet_usage;
pub use self::fleet_usage::*;

mod groups;
pub use self::groups::{JobGroupDimension, create_group_feature};

mod hierarchical_areas;
pub use self::hierarchical_areas::*;

mod known_edge;
pub use self::known_edge::create_known_edge_feature;

mod locked_jobs;
pub use self::locked_jobs::*;

mod minimize_overdue;
pub use self::minimize_overdue::*;

mod minimize_unassigned;
pub use self::minimize_unassigned::*;

mod reachable;
pub use self::reachable::create_reachable_feature;

mod recharge;
pub use self::recharge::RechargeFeatureBuilder;

mod reloads;
pub use self::reloads::{ReloadFeatureFactory, ReloadIntervalsTourState, SharedResource, SharedResourceId};

mod skills;
pub use self::skills::{JobSkills, JobSkillsDimension, VehicleSkillsDimension, create_skills_feature};

mod total_value;
pub use self::total_value::*;

mod tour_compactness;
pub use self::tour_compactness::*;

mod tour_limits;
pub use self::tour_limits::*;

mod tour_order;
pub use self::tour_order::*;

mod transport;
pub use self::transport::*;

mod vehicle_shifts;
pub use self::vehicle_shifts::*;

mod work_balance;
pub use self::work_balance::{
    create_activity_balanced_feature, create_distance_balanced_feature, create_duration_balanced_feature,
    create_max_load_balanced_feature,
};
