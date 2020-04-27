use super::Tuple;
use crate::models::common::MultiObjective;
use crate::models::common::Objective;
use std::cmp::Ordering;

pub struct Objective1;
pub struct Objective2;
pub struct Objective3;

impl Objective for Objective1 {
    type Solution = Tuple;

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

impl Objective for Objective2 {
    type Solution = Tuple;

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

impl Objective for Objective3 {
    type Solution = Tuple;

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

pub type TupleObjective = Box<dyn Objective<Solution = Tuple> + Send + Sync>;

pub struct TupleMultiObjective {
    objectives: Vec<TupleObjective>,
}

impl TupleMultiObjective {
    pub fn new(objectives: Vec<TupleObjective>) -> Self {
        Self { objectives }
    }
}

impl Objective for TupleMultiObjective {
    type Solution = Tuple;

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

impl MultiObjective for TupleMultiObjective {
    fn objectives<'a>(&'a self) -> Box<dyn Iterator<Item = &TupleObjective> + 'a> {
        Box::new(self.objectives.iter())
    }
}
