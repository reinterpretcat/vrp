//! Specifies different entities as extension points on Dimensions type.

use crate::constraints::{BreakPolicy, JobSkills};
use hashbrown::{HashMap, HashSet};
use vrp_core::models::common::{Dimensions, ValueDimension};

/// Specifies vehicle entity.
pub trait VehicleTie {
    /// Gets vehicle's id.
    fn get_vehicle_id(&self) -> Option<&String>;
    /// Sets vehicle's id.
    fn set_vehicle_id(&mut self, id: String) -> &mut Self;

    /// Gets vehicle's type id.
    fn get_vehicle_type(&self) -> Option<&String>;
    /// Sets vehicle's type id.
    fn set_vehicle_type(&mut self, id: String) -> &mut Self;

    /// Gets vehicle's shift.
    fn get_shift_index(&self) -> Option<usize>;
    /// Sets vehicle's shift.
    fn set_shift_index(&mut self, idx: usize) -> &mut Self;

    /// Gets vehicle's skills set.
    fn get_vehicle_skills(&self) -> Option<&HashSet<String>>;
    /// Sets vehicle's skills set.
    fn set_vehicle_skills(&mut self, skills: HashSet<String>) -> &mut Self;

    /// Gets vehicle's area.
    fn get_areas(&self) -> Option<&HashMap<String, (usize, f64)>>;
    /// Sets vehicle's area.
    fn set_areas(&mut self, areas: HashMap<String, (usize, f64)>) -> &mut Self;

    /// Gets vehicle's tour size.
    fn get_tour_size(&self) -> Option<usize>;
    /// Sets vehicle's tour size.
    fn set_tour_size(&mut self, tour_size: usize) -> &mut Self;
}

impl VehicleTie for Dimensions {
    fn get_vehicle_id(&self) -> Option<&String> {
        self.get_value("vehicle_id")
    }

    fn set_vehicle_id(&mut self, id: String) -> &mut Self {
        self.set_value("vehicle_id", id);
        self
    }

    fn get_vehicle_type(&self) -> Option<&String> {
        self.get_value("vehicle_type")
    }

    fn set_vehicle_type(&mut self, id: String) -> &mut Self {
        self.set_value("vehicle_type", id);
        self
    }

    fn get_shift_index(&self) -> Option<usize> {
        self.get_value("shift_index").cloned()
    }

    fn set_shift_index(&mut self, idx: usize) -> &mut Self {
        self.set_value("shift_index", idx);
        self
    }

    fn get_vehicle_skills(&self) -> Option<&HashSet<String>> {
        self.get_value("vehicle_skills")
    }

    fn set_vehicle_skills(&mut self, skills: HashSet<String>) -> &mut Self {
        self.set_value("vehicle_skills", skills);
        self
    }

    fn get_areas(&self) -> Option<&HashMap<String, (usize, f64)>> {
        self.get_value("areas")
    }

    fn set_areas(&mut self, areas: HashMap<String, (usize, f64)>) -> &mut Self {
        self.set_value("areas", areas);
        self
    }

    fn get_tour_size(&self) -> Option<usize> {
        self.get_value("tour_size").cloned()
    }

    fn set_tour_size(&mut self, tour_size: usize) -> &mut Self {
        self.set_value("tour_size", tour_size);
        self
    }
}

/// Specifies job entity.
pub trait JobTie {
    /// Gets job id.
    fn get_job_id(&self) -> Option<&String>;
    /// Sets job id.
    fn set_job_id(&mut self, id: String) -> &mut Self;

    /// Gets job skills.
    fn get_job_skills(&self) -> Option<&JobSkills>;
    /// Sets job skills.
    fn set_job_skills(&mut self, skills: Option<JobSkills>) -> &mut Self;

    /// Get job place tags.
    fn get_place_tags(&self) -> Option<&Vec<(usize, String)>>;
    /// Sets job place tags.
    fn set_place_tags(&mut self, tags: Option<Vec<(usize, String)>>) -> &mut Self;

    /// Gets job order.
    fn get_job_order(&self) -> Option<i32>;
    /// Sets job order.
    fn set_job_order(&mut self, order: Option<i32>) -> &mut Self;

    /// Gets job value.
    fn get_job_value(&self) -> Option<f64>;
    /// Sets job value.
    fn set_job_value(&mut self, value: Option<f64>) -> &mut Self;

    /// Gets job group.
    fn get_job_group(&self) -> Option<&String>;
    /// Sets job group.
    fn set_job_group(&mut self, group: Option<String>) -> &mut Self;

    /// Gets job compatibility.
    fn get_job_compatibility(&self) -> Option<&String>;
    /// Sets job compatibility.
    fn set_job_compatibility(&mut self, compatibility: Option<String>) -> &mut Self;

    /// Gets job (activity) type.
    fn get_job_type(&self) -> Option<&String>;
    /// Sets job (activity) type
    fn set_job_type(&mut self, job_type: String) -> &mut Self;
}

impl JobTie for Dimensions {
    fn get_job_id(&self) -> Option<&String> {
        self.get_value("job_id")
    }

    fn set_job_id(&mut self, id: String) -> &mut Self {
        self.set_value("job_id", id);
        self
    }

    fn get_job_skills(&self) -> Option<&JobSkills> {
        self.get_value("job_skills")
    }

    fn set_job_skills(&mut self, skills: Option<JobSkills>) -> &mut Self {
        if let Some(skills) = skills {
            self.set_value("job_skills", skills);
        } else {
            self.remove("job_skills");
        }

        self
    }

    fn get_place_tags(&self) -> Option<&Vec<(usize, String)>> {
        self.get_value("job_tags")
    }

    fn set_place_tags(&mut self, tags: Option<Vec<(usize, String)>>) -> &mut Self {
        if let Some(tags) = tags {
            self.set_value("job_tags", tags);
        } else {
            self.remove("job_tags");
        }

        self
    }

    fn get_job_order(&self) -> Option<i32> {
        self.get_value("job_order").cloned()
    }

    fn set_job_order(&mut self, order: Option<i32>) -> &mut Self {
        if let Some(order) = order {
            self.set_value("job_order", order);
        } else {
            self.remove("job_order");
        }

        self
    }

    fn get_job_value(&self) -> Option<f64> {
        self.get_value("job_value").cloned()
    }

    fn set_job_value(&mut self, value: Option<f64>) -> &mut Self {
        if let Some(value) = value {
            self.set_value("job_value", value);
        } else {
            self.remove("job_value");
        }

        self
    }

    fn get_job_group(&self) -> Option<&String> {
        self.get_value("job_group")
    }

    fn set_job_group(&mut self, group: Option<String>) -> &mut Self {
        if let Some(group) = group {
            self.set_value("job_group", group);
        } else {
            self.remove("job_group");
        }

        self
    }

    fn get_job_compatibility(&self) -> Option<&String> {
        self.get_value("job_compat")
    }

    fn set_job_compatibility(&mut self, compatibility: Option<String>) -> &mut Self {
        if let Some(compatibility) = compatibility {
            self.set_value("job_compat", compatibility);
        } else {
            self.remove("job_compat");
        }

        self
    }

    fn get_job_type(&self) -> Option<&String> {
        self.get_value("job_type")
    }

    fn set_job_type(&mut self, job_type: String) -> &mut Self {
        self.set_value("job_type", job_type);
        self
    }
}

/// Specifies break entity.
pub trait BreakTie {
    /// Gets break policy.
    fn get_break_policy(&self) -> Option<BreakPolicy>;
    /// Sets break policy.
    fn set_break_policy(&mut self, policy: BreakPolicy) -> &mut Self;
}

impl BreakTie for Dimensions {
    fn get_break_policy(&self) -> Option<BreakPolicy> {
        self.get_value("break_policy").cloned()
    }

    fn set_break_policy(&mut self, policy: BreakPolicy) -> &mut Self {
        self.set_value("break_policy", policy);
        self
    }
}
