//! Specifies different entities as extension points on Dimensions type.

use hashbrown::HashSet;
use vrp_core::construction::features::{BreakPolicy, JobSkills};

macro_rules! impl_dimension_property {
    ($trait_name:ident with $field_name:ident using $type:ty) => {
        paste::paste! {
            // define a dummy struct type which is used as a key
            struct [<$trait_name Key>];
            impl $trait_name for vrp_core::models::common::Dimensions {
                fn [<get_ $field_name>](&self) -> Option<&$type> {
                    self.get_value::<[<$trait_name Key>], _>()
                }

                fn [<set_ $field_name>](&mut self, value: $type) -> &mut Self {
                    self.set_value::<[<$trait_name Key>], _>(value);
                    self
                }
            }
        }
    };
}

/// Dimension to define a vehicle type property.
pub trait VehicleTypeDimension {
    /// Gets vehicle's type id.
    fn get_vehicle_type(&self) -> Option<&String>;
    /// Sets vehicle's type id.
    fn set_vehicle_type(&mut self, id: String) -> &mut Self;
}
impl_dimension_property!(VehicleTypeDimension with vehicle_type using String);

/// Dimension to define a vehicle shift index property.
pub trait ShiftIndexDimension {
    /// Gets vehicle's shift.
    fn get_shift_index(&self) -> Option<&usize>;
    /// Sets vehicle's shift.
    fn set_shift_index(&mut self, idx: usize) -> &mut Self;
}
impl_dimension_property!(ShiftIndexDimension with shift_index using usize);

/// Dimension to define a vehicle skills property.
pub trait VehicleSkillsDimension {
    /// Gets vehicle's skills set.
    fn get_vehicle_skills(&self) -> Option<&HashSet<String>>;
    /// Sets vehicle's skills set.
    fn set_vehicle_skills(&mut self, skills: HashSet<String>) -> &mut Self;
}
impl_dimension_property!(VehicleSkillsDimension with vehicle_skills using HashSet<String>);

/// Dimension to define a vehicle tour size property.
pub trait TourSizeDimension {
    /// Gets vehicle's tour size.
    fn get_tour_size(&self) -> Option<&usize>;
    /// Sets vehicle's tour size.
    fn set_tour_size(&mut self, tour_size: usize) -> &mut Self;
}
impl_dimension_property!(TourSizeDimension with tour_size using usize);

/// Dimension to define a job skills property.
pub trait JobSkillsDimension {
    /// Gets job skills.
    fn get_job_skills(&self) -> Option<&JobSkills>;
    /// Sets job skills.
    fn set_job_skills(&mut self, skills: JobSkills) -> &mut Self;
}
impl_dimension_property!(JobSkillsDimension with job_skills using JobSkills);

/// Dimension to define a job place tags property.
pub trait PlaceTagsDimension {
    /// Get job place tags.
    fn get_place_tags(&self) -> Option<&Vec<(usize, String)>>;
    /// Sets job place tags.
    fn set_place_tags(&mut self, tags: Vec<(usize, String)>) -> &mut Self;
}
impl_dimension_property!(PlaceTagsDimension with place_tags using Vec<(usize, String)>);

/// Dimension to define a job order property.
pub trait JobOrderDimension {
    /// Gets job order.
    fn get_job_order(&self) -> Option<&i32>;
    /// Sets job order.
    fn set_job_order(&mut self, order: i32) -> &mut Self;
}
impl_dimension_property!(JobOrderDimension with job_order using i32);

/// Dimension to define a job value property.
pub trait JobValueDimension {
    /// Gets job value.
    fn get_job_value(&self) -> Option<&f64>;
    /// Sets job value.
    fn set_job_value(&mut self, value: f64) -> &mut Self;
}
impl_dimension_property!(JobValueDimension with job_value using f64);

/// Dimension to define a job group property.
pub trait JobGroupDimension {
    /// Gets job group.
    fn get_job_group(&self) -> Option<&String>;
    /// Sets job group.
    fn set_job_group(&mut self, group: String) -> &mut Self;
}
impl_dimension_property!(JobGroupDimension with job_group using String);

/// Dimension to define a job compatibility property.
pub trait JobCompatibilityDimension {
    /// Gets job compatibility.
    fn get_job_compatibility(&self) -> Option<&String>;
    /// Sets job compatibility.
    fn set_job_compatibility(&mut self, compatibility: String) -> &mut Self;
}
impl_dimension_property!(JobCompatibilityDimension with job_compatibility using String);

/// Dimension to define a job type property.
pub trait JobTypeDimension {
    /// Gets job (activity) type.
    fn get_job_type(&self) -> Option<&String>;
    /// Sets job (activity) type
    fn set_job_type(&mut self, job_type: String) -> &mut Self;
}
impl_dimension_property!(JobTypeDimension with job_type using String);

/// Dimensions to define a break policy property.
pub trait BreakPolicyDimension {
    /// Gets break policy.
    fn get_break_policy(&self) -> Option<&BreakPolicy>;
    /// Sets break policy.
    fn set_break_policy(&mut self, policy: BreakPolicy) -> &mut Self;
}
impl_dimension_property!(BreakPolicyDimension with break_policy using BreakPolicy);
