extern crate rand;
use rand::seq::IteratorRandom;

use core::models::problem::{Multi, Single};
use std::sync::Arc;

#[cfg(test)]
#[path = "../../tests/unit/utils/permutations_test.rs"]
mod permutations_test;

pub fn get_permutations(size: usize) -> Permutations {
    Permutations { idxs: (0..size).collect(), swaps: vec![0; size], i: 0 }
}

pub struct Permutations {
    idxs: Vec<usize>,
    swaps: Vec<usize>,
    i: usize,
}

impl Iterator for Permutations {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i > 0 {
            loop {
                if self.i >= self.swaps.len() {
                    return None;
                }
                if self.swaps[self.i] < self.i {
                    break;
                }
                self.swaps[self.i] = 0;
                self.i += 1;
            }
            self.idxs.swap(self.i, (self.i & 1) * self.swaps[self.i]);
            self.swaps[self.i] += 1;
        }
        self.i = 1;
        Some(self.idxs.clone())
    }
}

// TODO move this with modification to support multi job with pickups and deliveries
fn get_job_permutations(multi: &Multi) -> Vec<Vec<Arc<Single>>> {
    // TODO optionally use permutation function defined on multi job
    // TODO configure sample size
    // TODO avoid extra memory allocations?
    const SAMPLE_SIZE: usize = 3;

    let mut rng = rand::thread_rng();
    get_permutations(multi.jobs.len())
        .choose_multiple(&mut rng, SAMPLE_SIZE)
        .iter()
        .map(|permutation| {
            permutation.iter().map(|&i| multi.jobs.get(i).unwrap().clone()).collect::<Vec<Arc<Single>>>()
        })
        .collect()
}
