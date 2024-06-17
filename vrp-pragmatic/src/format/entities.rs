//! Specifies different entities as extension points on Dimensions type.

use std::collections::HashSet;
use vrp_core::construction::features::{BreakPolicy, JobSkills};

macro_rules! custom_dimension {
    ($trait_name:ident with $field_name:ident using $type:ty) => {
        paste::paste! {
            /// A custom dimension.
            pub trait $trait_name {
                /// Gets custom property.
                fn [<get_ $field_name>](&self) -> Option<&$type>;
                /// Sets custom property.
                fn [<set_ $field_name>](&mut self, value: $type) -> &mut Self;
            }

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

custom_dimension!(VehicleTypeDimension with vehicle_type using String);

custom_dimension!(ShiftIndexDimension with shift_index using usize);

custom_dimension!(VehicleSkillsDimension with vehicle_skills using HashSet<String>);

custom_dimension!(TourSizeDimension with tour_size using usize);

custom_dimension!(JobSkillsDimension with job_skills using JobSkills);

custom_dimension!(PlaceTagsDimension with place_tags using Vec<(usize, String)>);

custom_dimension!(JobOrderDimension with job_order using i32);

custom_dimension!(JobValueDimension with job_value using f64);

custom_dimension!(JobGroupDimension with job_group using String);

custom_dimension!(JobCompatibilityDimension with job_compatibility using String);

custom_dimension!(JobTypeDimension with job_type using String);

custom_dimension!(BreakPolicyDimension with break_policy using BreakPolicy);
