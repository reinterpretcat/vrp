use crate::algorithms::nsga2::{dominance_order, MultiObjective, Objective};
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::sync::Arc;

pub type SliceObjective = Arc<dyn Objective<Solution = Vec<f64>> + Send + Sync>;

pub struct SliceDimensionObjective {
    dimension: usize,
}

impl SliceDimensionObjective {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Objective for SliceDimensionObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(a[self.dimension], b[self.dimension])
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a[self.dimension] - b[self.dimension]
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution[self.dimension]
    }
}

pub struct SliceSumObjective;

impl<'a> Objective for SliceSumObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(self.fitness(a), self.fitness(b))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        self.fitness(a) - self.fitness(b)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.iter().sum::<f64>()
    }
}

pub struct SliceMultiObjective {
    objectives: Vec<SliceObjective>,
}

impl SliceMultiObjective {
    pub fn new(objectives: Vec<SliceObjective>) -> Self {
        Self { objectives }
    }
}

impl Default for SliceMultiObjective {
    fn default() -> Self {
        SliceMultiObjective { objectives: Default::default() }
    }
}

impl Objective for SliceMultiObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        // TODO support more dimensions if necessary
        assert_eq!(a.len(), 2);
        assert_eq!(a.len(), b.len());

        if a[0] < b[0] && a[1] <= b[1] {
            Ordering::Less
        } else if a[0] <= b[0] && a[1] < b[1] {
            Ordering::Less
        } else if a[0] > b[0] && a[1] >= b[1] {
            Ordering::Greater
        } else if a[0] >= b[0] && a[1] > b[1] {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unimplemented!()
    }

    fn fitness(&self, _solution: &Self::Solution) -> f64 {
        unimplemented!()
    }
}

impl MultiObjective for SliceMultiObjective {
    fn objectives<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a (dyn Objective<Solution = Self::Solution> + Send + Sync)> + 'a> {
        Box::new(self.objectives.iter().map(|o| o.as_ref()))
    }
}

pub struct SliceHierarchicalObjective {
    primary_objectives: Vec<SliceObjective>,
    secondary_objectives: Vec<SliceObjective>,
}

impl SliceHierarchicalObjective {
    pub fn new(primary_objectives: Vec<SliceObjective>, secondary_objectives: Vec<SliceObjective>) -> Self {
        Self { primary_objectives, secondary_objectives }
    }
}

impl Objective for SliceHierarchicalObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        match dominance_order(a, b, &self.primary_objectives) {
            Ordering::Equal => dominance_order(a, b, &self.secondary_objectives),
            order => order,
        }
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unimplemented!()
    }

    fn fitness(&self, _solution: &Self::Solution) -> f64 {
        unimplemented!()
    }
}

impl MultiObjective for SliceHierarchicalObjective {
    fn objectives<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a (dyn Objective<Solution = Self::Solution> + Send + Sync)> + 'a> {
        Box::new(self.primary_objectives.iter().chain(self.secondary_objectives.iter()).map(|o| o.as_ref()))
    }
}
