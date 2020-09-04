use super::Pair;
use crate::algorithms::nsga2::{dominance_order, MultiObjective, Objective};
use crate::helpers::algorithms::nsga2::Triple;
use crate::utils::compare_floats;
use std::cmp::Ordering;

pub struct PairObjective1;
pub struct PairObjective2;
pub struct PairObjective3;

impl Objective for PairObjective1 {
    type Solution = Pair;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        a.0.cmp(&b.0)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.0 as f64) - (b.0 as f64)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.0 as f64
    }
}

impl Objective for PairObjective2 {
    type Solution = Pair;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        a.1.cmp(&b.1)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.1 as f64) - (b.1 as f64)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.1 as f64
    }
}

impl Objective for PairObjective3 {
    type Solution = Pair;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        (a.0 + a.1).cmp(&(b.0 + b.1))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.0 + a.1) as f64 - (b.0 + b.1) as f64
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        (solution.0 + solution.1) as f64
    }
}

pub type PairObjective = Box<dyn Objective<Solution = Pair> + Send + Sync>;

pub struct PairMultiObjective {
    objectives: Vec<PairObjective>,
}

impl PairMultiObjective {
    pub fn new(objectives: Vec<PairObjective>) -> Self {
        Self { objectives }
    }
}

impl Objective for PairMultiObjective {
    type Solution = Pair;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        if a.0 < b.0 && a.1 <= b.1 {
            Ordering::Less
        } else if a.0 <= b.0 && a.1 < b.1 {
            Ordering::Less
        } else if a.0 > b.0 && a.1 >= b.1 {
            Ordering::Greater
        } else if a.0 >= b.0 && a.1 > b.1 {
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

impl MultiObjective for PairMultiObjective {
    fn objectives<'a>(&'a self) -> Box<dyn Iterator<Item = &PairObjective> + 'a> {
        Box::new(self.objectives.iter())
    }
}

// TODO merge pair and triple objectives to have single common types

pub struct TripleObjective1;
pub struct TripleObjective2;
pub struct TripleObjective3;

impl Objective for TripleObjective1 {
    type Solution = Triple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(a.0, b.0)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a.0 - b.0
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.0
    }
}

impl Objective for TripleObjective2 {
    type Solution = Triple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(a.1, b.1)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a.1 - b.1
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.1
    }
}

impl Objective for TripleObjective3 {
    type Solution = Triple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(a.2, b.2)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a.2 - b.2
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.2
    }
}

pub type TripleObjective = Box<dyn Objective<Solution = Triple> + Send + Sync>;

pub struct TupleHierarchicalObjective {
    primary_objectives: Vec<TripleObjective>,
    secondary_objectives: Vec<TripleObjective>,
}

impl TupleHierarchicalObjective {
    pub fn new(primary_objectives: Vec<TripleObjective>, secondary_objectives: Vec<TripleObjective>) -> Self {
        Self { primary_objectives, secondary_objectives }
    }
}

impl Objective for TupleHierarchicalObjective {
    type Solution = Triple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        match dominance_order(a, b, &self.primary_objectives) {
            Ordering::Equal => dominance_order(a, b, &self.secondary_objectives),
            order => order,
        }
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unreachable!()
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.2
    }
}

impl MultiObjective for TupleHierarchicalObjective {
    fn objectives<'a>(&'a self) -> Box<dyn Iterator<Item = &TripleObjective> + 'a> {
        Box::new(self.primary_objectives.iter().chain(self.secondary_objectives.iter()))
    }
}
