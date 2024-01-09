//! Specifies different entities as extension points on Dimensions type.

use crate::construction::features::{BreakPolicy, JobSkills};
use hashbrown::HashSet;
use vrp_core::models::common::{DimenKey, Dimensions};

/// Specifies vehicle entity.
pub trait VehicleTie {
    /// Gets vehicle's id.
    fn get_vehicle_id(&self, key: DimenKey) -> Option<&String>;
    /// Sets vehicle's id.
    fn set_vehicle_id(&mut self, key: DimenKey, id: String) -> &mut Self;

    /// Gets vehicle's type id.
    fn get_vehicle_type(&self, key: DimenKey) -> Option<&String>;
    /// Sets vehicle's type id.
    fn set_vehicle_type(&mut self, key: DimenKey, id: String) -> &mut Self;

    /// Gets vehicle's shift.
    fn get_shift_index(&self, key: DimenKey) -> Option<usize>;
    /// Sets vehicle's shift.
    fn set_shift_index(&mut self, key: DimenKey, idx: usize) -> &mut Self;

    /// Gets vehicle's skills set.
    fn get_vehicle_skills(&self, key: DimenKey) -> Option<&HashSet<String>>;
    /// Sets vehicle's skills set.
    fn set_vehicle_skills(&mut self, key: DimenKey, skills: HashSet<String>) -> &mut Self;

    /// Gets vehicle's tour size.
    fn get_tour_size(&self, key: DimenKey) -> Option<usize>;
    /// Sets vehicle's tour size.
    fn set_tour_size(&mut self, key: DimenKey, tour_size: usize) -> &mut Self;
}

impl VehicleTie for Dimensions {
    fn get_vehicle_id(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_vehicle_id(&mut self, key: DimenKey, id: String) -> &mut Self {
        self.set_value(key, id.clone());
        // NOTE: core internally uses `id` to provide debug output
        self.set_value(key, id);
        self
    }

    fn get_vehicle_type(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_vehicle_type(&mut self, key: DimenKey, id: String) -> &mut Self {
        self.set_value(key, id);
        self
    }

    fn get_shift_index(&self, key: DimenKey) -> Option<usize> {
        self.get_value(key).cloned()
    }

    fn set_shift_index(&mut self, key: DimenKey, idx: usize) -> &mut Self {
        self.set_value(key, idx);
        self
    }

    fn get_vehicle_skills(&self, key: DimenKey) -> Option<&HashSet<String>> {
        self.get_value(key)
    }

    fn set_vehicle_skills(&mut self, key: DimenKey, skills: HashSet<String>) -> &mut Self {
        self.set_value(key, skills);
        self
    }

    fn get_tour_size(&self, key: DimenKey) -> Option<usize> {
        self.get_value(key).cloned()
    }

    fn set_tour_size(&mut self, key: DimenKey, tour_size: usize) -> &mut Self {
        self.set_value(key, tour_size);
        self
    }
}

/// Specifies job entity.
pub trait JobTie {
    /// Gets job id.
    fn get_job_id(&self, key: DimenKey) -> Option<&String>;
    /// Sets job id.
    fn set_job_id(&mut self, key: DimenKey, id: String) -> &mut Self;

    /// Gets job skills.
    fn get_job_skills(&self, key: DimenKey) -> Option<&JobSkills>;
    /// Sets job skills.
    fn set_job_skills(&mut self, key: DimenKey, skills: Option<JobSkills>) -> &mut Self;

    /// Get job place tags.
    fn get_place_tags(&self, key: DimenKey) -> Option<&Vec<(usize, String)>>;
    /// Sets job place tags.
    fn set_place_tags(&mut self, key: DimenKey, tags: Option<Vec<(usize, String)>>) -> &mut Self;

    /// Gets job order.
    fn get_job_order(&self, key: DimenKey) -> Option<i32>;
    /// Sets job order.
    fn set_job_order(&mut self, key: DimenKey, order: Option<i32>) -> &mut Self;

    /// Gets job value.
    fn get_job_value(&self, key: DimenKey) -> Option<f64>;
    /// Sets job value.
    fn set_job_value(&mut self, key: DimenKey, value: Option<f64>) -> &mut Self;

    /// Gets job group.
    fn get_job_group(&self, key: DimenKey) -> Option<&String>;
    /// Sets job group.
    fn set_job_group(&mut self, key: DimenKey, group: Option<String>) -> &mut Self;

    /// Gets job compatibility.
    fn get_job_compatibility(&self, key: DimenKey) -> Option<&String>;
    /// Sets job compatibility.
    fn set_job_compatibility(&mut self, key: DimenKey, compatibility: Option<String>) -> &mut Self;

    /// Gets job (activity) type.
    fn get_job_type(&self, key: DimenKey) -> Option<&String>;
    /// Sets job (activity) type
    fn set_job_type(&mut self, key: DimenKey, job_type: String) -> &mut Self;
}

impl JobTie for Dimensions {
    fn get_job_id(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_job_id(&mut self, key: DimenKey, id: String) -> &mut Self {
        self.set_value(key, id.clone());
        // NOTE: core internally uses `id` to provide debug output
        self.set_value(key, id);
        self
    }

    fn get_job_skills(&self, key: DimenKey) -> Option<&JobSkills> {
        self.get_value(key)
    }

    fn set_job_skills(&mut self, key: DimenKey, skills: Option<JobSkills>) -> &mut Self {
        if let Some(skills) = skills {
            self.set_value(key, skills);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_place_tags(&self, key: DimenKey) -> Option<&Vec<(usize, String)>> {
        self.get_value(key)
    }

    fn set_place_tags(&mut self, key: DimenKey, tags: Option<Vec<(usize, String)>>) -> &mut Self {
        if let Some(tags) = tags {
            self.set_value(key, tags);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_job_order(&self, key: DimenKey) -> Option<i32> {
        self.get_value(key).cloned()
    }

    fn set_job_order(&mut self, key: DimenKey, order: Option<i32>) -> &mut Self {
        if let Some(order) = order {
            self.set_value(key, order);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_job_value(&self, key: DimenKey) -> Option<f64> {
        self.get_value(key).cloned()
    }

    fn set_job_value(&mut self, key: DimenKey, value: Option<f64>) -> &mut Self {
        if let Some(value) = value {
            self.set_value(key, value);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_job_group(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_job_group(&mut self, key: DimenKey, group: Option<String>) -> &mut Self {
        if let Some(group) = group {
            self.set_value(key, group);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_job_compatibility(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_job_compatibility(&mut self, key: DimenKey, compatibility: Option<String>) -> &mut Self {
        if let Some(compatibility) = compatibility {
            self.set_value(key, compatibility);
        } else {
            self.set_value(key, ());
        }

        self
    }

    fn get_job_type(&self, key: DimenKey) -> Option<&String> {
        self.get_value(key)
    }

    fn set_job_type(&mut self, key: DimenKey, job_type: String) -> &mut Self {
        self.set_value(key, job_type);
        self
    }
}

/// Specifies break entity.
pub trait BreakTie {
    /// Gets break policy.
    fn get_break_policy(&self, key: DimenKey) -> Option<BreakPolicy>;
    /// Sets break policy.
    fn set_break_policy(&mut self, key: DimenKey, policy: BreakPolicy) -> &mut Self;
}

impl BreakTie for Dimensions {
    fn get_break_policy(&self, key: DimenKey) -> Option<BreakPolicy> {
        self.get_value(key).cloned()
    }

    fn set_break_policy(&mut self, key: DimenKey, policy: BreakPolicy) -> &mut Self {
        self.set_value(key, policy);
        self
    }
}
