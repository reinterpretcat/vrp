use crate::algorithms::nsga2::{dominance_order, MultiObjective, Objective};
use crate::prelude::*;
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

impl Objective for SliceSumObjective {
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

#[derive(Default)]
pub struct SliceMultiObjective {
    objectives: Vec<SliceObjective>,
}

impl SliceMultiObjective {
    pub fn new(objectives: Vec<SliceObjective>) -> Self {
        Self { objectives }
    }
}

impl MultiObjective for SliceMultiObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        // TODO support more dimensions if necessary
        assert_eq!(a.len(), 2);
        assert_eq!(a.len(), b.len());

        if a[0] < b[0] && a[1] <= b[1] || a[0] <= b[0] && a[1] < b[1] {
            Ordering::Less
        } else if a[0] > b[0] && a[1] >= b[1] || a[0] >= b[0] && a[1] > b[1] {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.objectives.iter().map(|o| o.fitness(solution)))
    }

    fn get_order(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<Ordering, GenericError> {
        self.objectives.get(idx).map(|o| o.total_order(a, b)).ok_or_else(|| format!("wrong index: {idx}").into())
    }

    fn get_distance(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<f64, GenericError> {
        self.objectives.get(idx).map(|o| o.distance(a, b)).ok_or_else(|| format!("wrong index: {idx}").into())
    }

    fn size(&self) -> usize {
        self.objectives.len()
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

impl MultiObjective for SliceHierarchicalObjective {
    type Solution = Vec<f64>;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        match dominance_order(a, b, self.primary_objectives.iter().map(|o| o.as_ref())) {
            Ordering::Equal => dominance_order(a, b, self.secondary_objectives.iter().map(|o| o.as_ref())),
            order => order,
        }
    }

    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.primary_objectives.iter().chain(self.secondary_objectives.iter()).map(|o| o.fitness(solution)))
    }

    fn get_order(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<Ordering, GenericError> {
        self.primary_objectives
            .iter()
            .chain(self.secondary_objectives.iter())
            .nth(idx)
            .map(|o| o.total_order(a, b))
            .ok_or_else(|| format!("wrong index: {idx}").into())
    }

    fn get_distance(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<f64, GenericError> {
        self.primary_objectives
            .iter()
            .chain(self.secondary_objectives.iter())
            .nth(idx)
            .map(|o| o.distance(a, b))
            .ok_or_else(|| format!("wrong index: {idx}").into())
    }

    fn size(&self) -> usize {
        self.primary_objectives.len() + self.secondary_objectives.len()
    }
}
