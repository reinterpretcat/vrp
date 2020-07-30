mod single {
    use crate::models::common::{Capacity, SingleDimCapacity};

    fn from_value(capacity: i32) -> SingleDimCapacity {
        SingleDimCapacity::new(capacity)
    }

    #[test]
    fn can_sum_dimens() {
        assert_eq!(from_value(1) + from_value(2), from_value(3));
        assert_eq!(from_value(1) + from_value(0), from_value(1));

        assert_eq!(SingleDimCapacity::default() + from_value(0), SingleDimCapacity::default());
        assert_eq!(SingleDimCapacity::default() + SingleDimCapacity::default(), SingleDimCapacity::default());
    }

    #[test]
    fn can_sub_dimens() {
        assert_eq!(from_value(3) - from_value(2), from_value(1));
        assert_eq!(from_value(1) - from_value(0), from_value(1));

        assert_eq!(SingleDimCapacity::default() - from_value(0), SingleDimCapacity::default());
        assert_eq!(SingleDimCapacity::default() - SingleDimCapacity::default(), SingleDimCapacity::default());
    }

    #[test]
    fn can_compare_dimens() {
        assert!(from_value(2) > from_value(1));
        assert!(from_value(1) < from_value(3));
        assert!(from_value(5) >= from_value(2));

        assert!(from_value(2) < from_value(5));

        assert_eq!(from_value(0), SingleDimCapacity::default());
        assert_eq!(SingleDimCapacity::default(), SingleDimCapacity::default());
    }

    #[test]
    fn can_use_specific_functions() {
        assert!(from_value(1).is_not_empty());
        assert!(!from_value(0).is_not_empty());

        assert_eq!(from_value(10).max_load(from_value(5)), from_value(10));
        assert_eq!(from_value(5).max_load(from_value(10)), from_value(10));

        assert!(from_value(10).can_load(&from_value(5)));
        assert!(!from_value(5).can_load(&from_value(10)));
    }
}

mod multi {
    use crate::models::common::{Capacity, MultiDimCapacity};

    fn from_vec(capacity: Vec<i32>) -> MultiDimCapacity {
        MultiDimCapacity::new(capacity)
    }

    #[test]
    fn can_sum_dimens() {
        assert_eq!((from_vec(vec![1, 0, 2]) + from_vec(vec![3, 1, 0])), from_vec(vec![4, 1, 2]));
        assert_eq!((from_vec(vec![1, 0, 0]) + from_vec(vec![0, 0, 0])), from_vec(vec![1, 0, 0]));

        assert_eq!((from_vec(vec![1]) + from_vec(vec![0, 0, 2])), from_vec(vec![1, 0, 2]));
        assert_eq!((from_vec(vec![0, 0, 2]) + from_vec(vec![1])), from_vec(vec![1, 0, 2]));

        assert_ne!((from_vec(vec![1, 0, 2]) + from_vec(vec![3, 1, 0])), from_vec(vec![3, 1, 2]));
    }

    #[test]
    fn can_sub_dimens() {
        assert_eq!((from_vec(vec![3, 0, 2]) - from_vec(vec![1, 1, 4])), from_vec(vec![2, -1, -2]));
        assert_eq!((from_vec(vec![3, 0, 2]) - from_vec(vec![0, 0, 0])), from_vec(vec![3, 0, 2]));

        assert_eq!((from_vec(vec![1]) - from_vec(vec![0, 0, 2])), from_vec(vec![1, 0, -2]));
        assert_eq!((from_vec(vec![0, 0, 2]) - from_vec(vec![1])), from_vec(vec![-1, 0, 2]));

        assert_ne!((from_vec(vec![1, 0, 2]) - from_vec(vec![3, 1, 0])), from_vec(vec![3, 1, 2]));
    }

    #[test]
    fn can_compare_dimens() {
        assert!(from_vec(vec![3, 0, 2]) > from_vec(vec![1, 1, 4]));
        assert!(from_vec(vec![1, 0, 2]) < from_vec(vec![3, 3, 3]));
        assert!(from_vec(vec![3]) > from_vec(vec![1, 1, 4]));
        assert!(from_vec(vec![3]) < from_vec(vec![4, 1, 2]));

        assert_eq!(from_vec(vec![0, 0, 2]), from_vec(vec![0, 0, 2]));
        assert_eq!(from_vec(vec![1]), from_vec(vec![1, 0, 0]));
        assert_eq!(from_vec(vec![1, 0, 0]), from_vec(vec![1]));

        assert_eq!(from_vec(vec![0, 0, 0]), MultiDimCapacity::default());
    }

    #[test]
    fn can_use_specific_functions() {
        assert!(from_vec(vec![1, 0]).is_not_empty());
        assert!(!from_vec(vec![0, 0]).is_not_empty());

        assert_eq!(from_vec(vec![0, 1]).max_load(from_vec(vec![1, 0])), from_vec(vec![1, 1]));
        assert_eq!(from_vec(vec![3, 0, 2]).max_load(from_vec(vec![1, 1, 4])), from_vec(vec![3, 1, 4]));

        assert!(!from_vec(vec![1, 0]).can_load(&from_vec(vec![0, 1])));
        assert!(!from_vec(vec![3, 0, 2]).can_load(&from_vec(vec![1, 1, 4])));
    }
}
