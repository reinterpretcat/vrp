use crate::construction::states::SolutionContext;
use crate::models::solution::Route;
use crate::models::Solution;
use std::ops::Deref;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

pub trait SolomonSolution {
    fn into_solomon_solution(self) -> Solution;
}

impl SolomonSolution for SolutionContext {
    fn into_solomon_solution(self) -> Solution {
        Solution {
            registry: self.registry,
            routes: self
                .routes
                .into_iter()
                .map(|rc| Arc::try_unwrap(rc.route).unwrap_or_else(|_| panic!()).into_inner().unwrap())
                .collect(),
            unassigned: self.unassigned,
            extras: Default::default(),
        }
    }
}

struct SolomonWriter {}

impl SolomonWriter {
    pub fn write_solution(solution: &Solution) {}
}
