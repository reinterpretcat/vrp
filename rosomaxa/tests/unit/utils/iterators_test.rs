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
