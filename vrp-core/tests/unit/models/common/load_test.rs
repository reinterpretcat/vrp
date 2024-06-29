mod single {
    use crate::models::common::{Demand, Load, SingleDimLoad};

    fn from_value(load: i32) -> SingleDimLoad {
        SingleDimLoad::new(load)
    }

    #[test]
    fn can_sum_dimens() {
        assert_eq!(from_value(1) + from_value(2), from_value(3));
        assert_eq!(from_value(1) + from_value(0), from_value(1));

        assert_eq!(SingleDimLoad::default() + from_value(0), SingleDimLoad::default());
        assert_eq!(SingleDimLoad::default() + SingleDimLoad::default(), SingleDimLoad::default());
    }

    #[test]
    fn can_sub_dimens() {
        assert_eq!(from_value(3) - from_value(2), from_value(1));
        assert_eq!(from_value(1) - from_value(0), from_value(1));

        assert_eq!(SingleDimLoad::default() - from_value(0), SingleDimLoad::default());
        assert_eq!(SingleDimLoad::default() - SingleDimLoad::default(), SingleDimLoad::default());
    }

    #[test]
    fn can_compare_dimens() {
        assert!(from_value(2) > from_value(1));
        assert!(from_value(1) < from_value(3));
        assert!(from_value(5) >= from_value(2));

        assert!(from_value(2) < from_value(5));

        assert_eq!(from_value(0), SingleDimLoad::default());
        assert_eq!(SingleDimLoad::default(), SingleDimLoad::default());
    }

    #[test]
    fn can_use_specific_functions() {
        assert!(from_value(1).is_not_empty());
        assert!(!from_value(0).is_not_empty());

        assert_eq!(from_value(10).max_load(from_value(5)), from_value(10));
        assert_eq!(from_value(5).max_load(from_value(10)), from_value(10));

        assert!(from_value(10).can_fit(&from_value(5)));
        assert!(!from_value(5).can_fit(&from_value(10)));
    }

    #[test]
    fn can_use_pudo_simple_ctors() {
        let pickup = Demand::pickup(1);
        assert_eq!(pickup.pickup.0, SingleDimLoad::new(1));
        assert_eq!(pickup.pickup.1, SingleDimLoad::default());
        assert_eq!(pickup.delivery.0, SingleDimLoad::default());
        assert_eq!(pickup.delivery.1, SingleDimLoad::default());

        let dropoff = Demand::delivery(1);
        assert_eq!(dropoff.pickup.0, SingleDimLoad::default());
        assert_eq!(dropoff.pickup.1, SingleDimLoad::default());
        assert_eq!(dropoff.delivery.0, SingleDimLoad::new(1));
        assert_eq!(dropoff.delivery.1, SingleDimLoad::default());
    }

    #[test]
    fn can_use_pudo_demand_ctors() {
        let pickup = Demand::pudo_pickup(1);
        assert_eq!(pickup.pickup.0, SingleDimLoad::default());
        assert_eq!(pickup.pickup.1, SingleDimLoad::new(1));
        assert_eq!(pickup.delivery.0, SingleDimLoad::default());
        assert_eq!(pickup.delivery.1, SingleDimLoad::default());

        let dropoff = Demand::pudo_delivery(1);
        assert_eq!(dropoff.pickup.0, SingleDimLoad::default());
        assert_eq!(dropoff.pickup.1, SingleDimLoad::default());
        assert_eq!(dropoff.delivery.0, SingleDimLoad::default());
        assert_eq!(dropoff.delivery.1, SingleDimLoad::new(1));
    }
}

mod multi {
    use crate::models::common::{Load, MultiDimLoad};
    use std::cmp::Ordering;

    fn from_vec(load: Vec<i32>) -> MultiDimLoad {
        MultiDimLoad::new(load)
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
        assert_eq!(from_vec(vec![3, 0, 2]).partial_cmp(&from_vec(vec![1, 1, 4])), None);
        assert_eq!(from_vec(vec![1, 0, 2]).partial_cmp(&from_vec(vec![3, 3, 3])), Some(Ordering::Less));
        assert_eq!(from_vec(vec![3, 3, 3]).partial_cmp(&from_vec(vec![1, 0, 2])), Some(Ordering::Greater));

        assert_eq!(from_vec(vec![3]).partial_cmp(&from_vec(vec![1, 1, 4])), None);

        assert_eq!(from_vec(vec![0, 0, 2]).partial_cmp(&from_vec(vec![0, 0, 2])), Some(Ordering::Equal));
        assert_eq!(from_vec(vec![1, 0, 0]).partial_cmp(&from_vec(vec![1])), Some(Ordering::Equal));
        assert_eq!(from_vec(vec![0, 0, 0]).partial_cmp(&MultiDimLoad::default()), Some(Ordering::Equal));
    }

    #[test]
    fn can_use_specific_functions() {
        assert!(from_vec(vec![1, 0]).is_not_empty());
        assert!(!from_vec(vec![0, 0]).is_not_empty());

        assert_eq!(from_vec(vec![0, 1]).max_load(from_vec(vec![1, 0])), from_vec(vec![1, 1]));
        assert_eq!(from_vec(vec![3, 0, 2]).max_load(from_vec(vec![1, 1, 4])), from_vec(vec![3, 1, 4]));

        assert!(!from_vec(vec![1, 0]).can_fit(&from_vec(vec![0, 1])));
        assert!(!from_vec(vec![3, 0, 2]).can_fit(&from_vec(vec![1, 1, 4])));
    }
}
