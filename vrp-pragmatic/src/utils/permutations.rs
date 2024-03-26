use hashbrown::HashSet;
use rand::prelude::SliceRandom;
use vrp_core::models::problem::JobPermutation;
use vrp_core::prelude::*;

#[cfg(test)]
#[path = "../../tests/unit/utils/permutations_test.rs"]
mod permutations_test;

pub struct VariableJobPermutation {
    size: usize,
    split_start_index: usize,
    sample_size: usize,
    random: Random,
}

impl VariableJobPermutation {
    pub fn new(size: usize, split_start_index: usize, sample_size: usize, random: Random) -> Self {
        assert!(size > 0);
        Self { size, split_start_index, sample_size, random }
    }
}

impl JobPermutation for VariableJobPermutation {
    fn get(&self) -> Vec<Vec<usize>> {
        get_split_permutations(self.size, self.split_start_index, self.sample_size, &self.random)
    }

    fn validate(&self, permutation: &[usize]) -> bool {
        permutation.iter().cloned().collect::<HashSet<_>>().len() == self.size
            && permutation[0..self.split_start_index].iter().max().map_or(false, |&max| max < self.split_start_index)
            && permutation[self.split_start_index..].iter().min().map_or(false, |&min| min >= self.split_start_index)
    }
}

fn generate_sample_permutations(start: usize, end: usize, sample_size: usize, random: &Random) -> Vec<Vec<usize>> {
    // NOTE prevent to have more then possible unique permutations for simple cases
    let size = end - start + 1;
    let sample_size = if size < 10 {
        let total_permutations = (1..=size).product();
        sample_size.min(total_permutations)
    } else {
        sample_size
    };

    let data = (start..=end).collect::<Vec<_>>();
    let mut result = vec![data; sample_size];
    let mut rng = random.get_rng();
    result.iter_mut().for_each(|data| {
        data.shuffle(&mut rng);
    });

    result
}

fn get_split_permutations(
    size: usize,
    split_start_index: usize,
    sample_size: usize,
    random: &Random,
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
