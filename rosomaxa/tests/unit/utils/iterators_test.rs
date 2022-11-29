use super::*;
use crate::utils::DefaultRandom;

mod selection_sampling {
    use super::*;

    #[test]
    fn can_sample_from_large_range() {
        let random = Arc::new(DefaultRandom::default());
        let amount = 5;

        let numbers = SelectionSamplingIterator::new(0..100, amount, random).collect::<Vec<_>>();

        assert_eq!(numbers.len(), amount);
        numbers.windows(2).for_each(|item| match item {
            &[prev, next] => assert!(prev < next),
            _ => unreachable!(),
        });
        numbers.windows(2).any(|item| match item {
            &[prev, next] => prev + 1 < next,
            _ => false,
        });
    }

    #[test]
    fn can_sample_from_same_range() {
        let amount = 5;
        let random = Arc::new(DefaultRandom::default());

        let numbers = SelectionSamplingIterator::new(0..amount, amount, random).collect::<Vec<_>>();

        assert_eq!(numbers, vec![0, 1, 2, 3, 4])
    }

    #[test]
    fn can_sample_from_smaller_range() {
        let sample_size = 5;
        let random = Arc::new(DefaultRandom::default());

        let numbers = create_range_sampling_iter(0..3, sample_size, random.as_ref()).collect::<Vec<_>>();

        assert_eq!(numbers, vec![0, 1, 2])
    }
}

mod range_sampling {
    use super::*;
    use crate::prelude::RandomGen;

    struct DummyRandom {
        value: i32,
    }
    impl Random for DummyRandom {
        fn uniform_int(&self, min: i32, max: i32) -> i32 {
            assert!((min..=max).contains(&self.value));

            self.value
        }

        fn uniform_real(&self, _: f64, _: f64) -> f64 {
            unimplemented!()
        }

        fn is_head_not_tails(&self) -> bool {
            unimplemented!()
        }

        fn is_hit(&self, _: f64) -> bool {
            unimplemented!()
        }

        fn weighted(&self, _: &[usize]) -> usize {
            unimplemented!()
        }

        fn get_rng(&self) -> RandomGen {
            unimplemented!()
        }
    }

    #[test]
    fn can_sample_from_large_range() {
        let sample_size = 5;
        let random = DummyRandom { value: 1 };

        let numbers = create_range_sampling_iter(0..100, sample_size, &random).collect::<Vec<_>>();

        assert_eq!(numbers, vec![5, 6, 7, 8, 9])
    }

    #[test]
    fn can_sample_from_same_range() {
        let sample_size = 5;
        let random = Arc::new(DefaultRandom::default());

        let numbers = create_range_sampling_iter(0..5, sample_size, random.as_ref()).collect::<Vec<_>>();

        assert_eq!(numbers, vec![0, 1, 2, 3, 4])
    }

    #[test]
    fn can_sample_from_smaller_range() {
        let sample_size = 5;
        let random = Arc::new(DefaultRandom::default());

        let numbers = create_range_sampling_iter(0..3, sample_size, random.as_ref()).collect::<Vec<_>>();

        assert_eq!(numbers, vec![0, 1, 2])
    }
}

mod sampling_search {
    use super::*;
    use crate::Environment;
    use std::sync::RwLock;

    #[derive(Clone, Debug, Default)]
    struct DataType {
        data: bool,
        idx: i32,
    }

    use crate::prelude::RandomGen;

    struct DummyRandom {
        target_sequences: Vec<Vec<usize>>,
        current_sequence: RwLock<usize>,
        current_item: RwLock<usize>,
        sampled: RwLock<usize>,
        sample: usize,
        total: usize,
    }

    impl DummyRandom {
        pub fn new(sample: usize, total: usize, target_sequences: Vec<Vec<usize>>) -> Self {
            Self {
                target_sequences,
                current_sequence: RwLock::new(0),
                current_item: RwLock::new(0),
                sampled: RwLock::new(0),
                sample,
                total,
            }
        }
    }

    impl Random for DummyRandom {
        fn uniform_int(&self, _: i32, _: i32) -> i32 {
            unimplemented!()
        }

        fn uniform_real(&self, _: f64, _: f64) -> f64 {
            unimplemented!()
        }

        fn is_head_not_tails(&self) -> bool {
            unimplemented!()
        }

        fn is_hit(&self, _: f64) -> bool {
            if *self.current_item.write().unwrap() >= self.total {
                *self.current_item.write().unwrap() = 0;
                *self.sampled.write().unwrap() = 0;
                *self.current_sequence.write().unwrap() += 1;
            }

            let target_sequence_idx = *self.current_sequence.read().unwrap();
            let target_sequence = if let Some(target_sequence) = self.target_sequences.get(target_sequence_idx) {
                target_sequence
            } else {
                return false;
            };
            let current = *self.current_item.read().unwrap();

            *self.current_item.write().unwrap() += 1;

            if target_sequence.contains(&current) {
                *self.sampled.write().unwrap() += 1;

                if *self.sampled.write().unwrap() == self.sample {
                    *self.current_item.write().unwrap() = self.total;
                }
                true
            } else {
                false
            }
        }

        fn weighted(&self, _: &[usize]) -> usize {
            unimplemented!()
        }

        fn get_rng(&self) -> RandomGen {
            unimplemented!()
        }
    }

    #[allow(clippy::type_complexity)]
    fn get_result_comparer(target: i32) -> Box<dyn Fn(&DataType, &DataType) -> bool> {
        Box::new(move |left, right| {
            match (left.data, right.data) {
                (true, false) => return true,
                (false, true) => return false,
                _ => {}
            }
            match (left.idx, right.idx) {
                (_, rhs) if rhs == target => false,
                (lhs, _) if lhs == target => true,
                (lhs, rhs) => (lhs - target).abs() < (rhs - target).abs(),
            }
        })
    }

    parameterized_test! {can_redefine_random_as_expected, (sample, total, target_sequence), {
        can_redefine_random_as_expected_impl(sample, total, target_sequence);
    }}

    can_redefine_random_as_expected! {
         case_01: (3, 10, vec![vec![3, 5, 9]]),
         case_02: (3, 10, vec![vec![3, 5, 9], vec![6, 7, 8]]),
         case_03: (3, 10, vec![vec![2, 3, 5], vec![6, 7, 8]]),
         case_04: (3, 10, vec![vec![2, 3, 5], vec![6, 7, 9]]),
         case_05: (4, 100, vec![vec![3, 5, 17, 96]]),
    }

    fn can_redefine_random_as_expected_impl(sample: usize, total: usize, target_sequences: Vec<Vec<usize>>) {
        let random = Arc::new(DummyRandom::new(sample, total, target_sequences.clone()));

        target_sequences.into_iter().for_each(|target_sequence| {
            let results = SelectionSamplingIterator::new(0..total, sample, random.clone()).collect::<Vec<_>>();
            assert_eq!(results, target_sequence);
        });
    }

    parameterized_test! {can_search_for_best, (skip, target, target_sequences), {
        can_search_for_best_impl(skip, target, target_sequences);
    }}

    can_search_for_best! {
        case_01: (0, 10, vec![(1, vec![3, 22, 45, 96]), (0, vec![4, 15, 32, 44])]),
        case_02: (0, 10, vec![(2, vec![1, 21, 46, 96]), (2, vec![23, 25, 32, 45])]),
        case_03: (0, 10, vec![(1, vec![1, 4, 46, 96]), (1, vec![3, 8, 20, 45]), (0, vec![10, 12, 17, 19])]),

        case_04_should_not_use_second: (0, 10, vec![(1, vec![1, 2, 3, 96]), (usize::MAX, vec![2, 2, 2, 2])]),
    }

    fn can_search_for_best_impl(skip: usize, target: i32, target_sequences: Vec<(usize, Vec<usize>)>) {
        let total_size = 100;
        let sample_size = 4;
        let expected_idx = *target_sequences.iter().rev().find_map(|(idx, data)| data.get(*idx)).unwrap();
        let target_sequences = target_sequences
            .iter()
            .enumerate()
            .rev()
            .map(|(idx, (_, target_sequence))| {
                if idx != 0 {
                    let (offset_idx, prev_sequence) = target_sequences.get(idx - 1).unwrap();
                    let offset = if *offset_idx != 0 { prev_sequence[*offset_idx - 1] } else { 0 } + 1;
                    target_sequence.iter().map(|target| *target - offset).collect()
                } else {
                    target_sequence.clone()
                }
            })
            .rev()
            .collect();
        let random = Arc::new(DummyRandom::new(sample_size, total_size, target_sequences));
        let map_fn = |item: &DataType| item.clone();
        let compare_fn = get_result_comparer(target);
        let data = (0..total_size).map(|idx| DataType { data: idx % 2 == 0, idx: idx as i32 }).collect::<Vec<_>>();

        let element =
            data.iter().skip(skip).sample_search(sample_size, random, map_fn, |item| item.idx, compare_fn).unwrap();

        assert_eq!(element.idx as usize, expected_idx);
    }

    #[test]
    fn can_keep_evaluations_amount_low() {
        let total_size = 1000;
        let sample_size = 8;
        let target = 10;
        let random = Environment::default().random;

        let mut results = (0..100)
            .map(|_| {
                let counter = RwLock::new(0);
                let map_fn = |item: &DataType| {
                    *counter.write().unwrap() += 1;
                    item.clone()
                };
                let compare_fn = get_result_comparer(target);
                let data =
                    (0..total_size).map(|idx| DataType { data: idx % 2 == 0, idx: idx as i32 }).collect::<Vec<_>>();

                let idx = data
                    .iter()
                    .sample_search(sample_size, random.clone(), map_fn, |item| item.idx, compare_fn)
                    .unwrap()
                    .idx;
                let count = *counter.read().unwrap();
                (idx, count)
            })
            .collect::<Vec<_>>();

        results.sort_by(|(a, _), (b, _)| a.cmp(b));
        let median = results[results.len() / 2];
        assert!(median.0 < 250);
        assert!(results.iter().all(|(_, count)| *count < 100));
    }
}
