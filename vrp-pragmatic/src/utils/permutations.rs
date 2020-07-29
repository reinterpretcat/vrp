use rand::seq::IteratorRandom;
use std::collections::HashSet;
use std::sync::Arc;
use vrp_core::models::problem::JobPermutation;
use vrp_core::utils::Random;

#[cfg(test)]
#[path = "../../tests/unit/utils/permutations_test.rs"]
mod permutations_test;

pub struct VariableJobPermutation {
    size: usize,
    split_start_index: usize,
    sample_size: usize,
    random: Arc<dyn Random + Sync + Send>,
}

impl VariableJobPermutation {
    pub fn new(
        size: usize,
        split_start_index: usize,
        sample_size: usize,
        random: Arc<dyn Random + Sync + Send>,
    ) -> Self {
        assert!(size > 0);
        Self { size, split_start_index, sample_size, random }
    }
}

impl JobPermutation for VariableJobPermutation {
    fn get(&self) -> Vec<Vec<usize>> {
        get_split_permutations(self.size, self.split_start_index, self.sample_size, self.random.as_ref())
    }

    fn validate(&self, permutation: &[usize]) -> bool {
        permutation.iter().cloned().collect::<HashSet<_>>().len() == self.size
            && permutation[0..self.split_start_index].iter().max().map_or(false, |&max| max < self.split_start_index)
            && permutation[self.split_start_index..].iter().min().map_or(false, |&min| min >= self.split_start_index)
    }
}

fn get_permutations(start: usize, end: usize) -> Permutations {
    Permutations { idxs: (start..=end).collect(), swaps: vec![0; end - start + 1], i: 0 }
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

fn generate_sample_permutations(
    start: usize,
    end: usize,
    sample_size: usize,
    random: &(dyn Random + Sync + Send),
) -> Vec<Vec<usize>> {
    get_permutations(start, end)
        .choose_multiple(&mut random.get_rng(), sample_size)
        .iter()
        .map(|permutation| permutation.iter().copied().collect::<Vec<usize>>())
        .collect()
}

fn get_split_permutations(
    size: usize,
    split_start_index: usize,
    sample_size: usize,
    random: &(dyn Random + Sync + Send),
) -> Vec<Vec<usize>> {
    // TODO make it memory efficient somehow

    match split_start_index {
        x if x == 0 || x == size => generate_sample_permutations(0, size - 1, sample_size, random),
        _ => {
            assert!(size > split_start_index);

            let first = generate_sample_permutations(0, split_start_index - 1, sample_size, random);
            let second = generate_sample_permutations(split_start_index, size - 1, sample_size, random);

            first
                .iter()
                .flat_map(|a| {
                    second
                        .iter()
                        .map(|b| a.iter().chain(b.iter()).cloned().collect::<Vec<usize>>())
                        .collect::<Vec<Vec<usize>>>()
                })
                .take(sample_size)
                .collect()
        }
    }
}
