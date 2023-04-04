use super::*;

mod costs {
    use super::*;

    fn make(data: &[Cost]) -> InsertionCost {
        InsertionCost::new(data)
    }

    #[test]
    fn can_use_big_sizes_for_insertion_costs() {
        let cost = make(&[0., 0., 1., 0., 0., 2., 0., 0., 3., 0., 0., 4.]);

        assert_eq!(cost.data[0], 0.);
        assert_eq!(cost.data[1], 0.);
        assert_eq!(cost.data[2], 1.);
        assert_eq!(cost.data[3], 0.);
        assert_eq!(cost.data[5], 2.);
        assert_eq!(cost.data[8], 3.);
        assert_eq!(cost.data[11], 4.);
    }

    #[test]
    fn can_compare_insertion_costs() {
        assert_eq!(make(&[1., 0., 0.]), make(&[1., 0., 0.]));
        assert_eq!(make(&[0., 1., 0.]), make(&[0., 1., 0.]));
        assert_eq!(make(&[0., 1.]), make(&[0., 1., 0.]));
        assert_eq!(make(&[0., 1., 0.]), make(&[0., 1.]));
        assert_eq!(make(&[0., 0., 1.]), make(&[0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0.]));

        assert!(make(&[0., 1., 0.]) > make(&[0., 0., 0.]));
        assert!(make(&[0., 0., 0.]) < make(&[0., 1., 0.]));

        assert!(make(&[0., 1.]) < make(&[0., 1., 1.]));
        assert!(make(&[0., 0., 1.]) < make(&[0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 1.]));
    }

    #[test]
    fn can_compare_insertion_cost_defaults() {
        assert!(InsertionCost::default() < make(&[0., 1.]));
        assert_eq!(InsertionCost::default(), make(&[0., 0.]));
        assert!(InsertionCost::default() > make(&[-1., 0.]));
    }

    #[test]
    fn can_sum_costs() {
        assert_eq!(make(&[1., 0., 0.]) + make(&[1., 0., 0.]), make(&[2., 0., 0.]));
        assert_eq!(make(&[1., 0., 0.]) + make(&[0., 1., 0.]), make(&[1., 1., 0.]));

        assert_eq!(make(&[1., 0.]) + make(&[0., 1., 0.]), make(&[1., 1., 0.]));
        assert_eq!(make(&[0., 0., 1.]) + make(&[1., 0.]), make(&[1., 0., 1.]));
    }

    #[test]
    fn can_sum_defaults() {
        assert_eq!(InsertionCost::default() + make(&[0., 1.]), make(&[0., 1.]));
        assert_eq!(make(&[0., 1.]) + InsertionCost::default(), make(&[0., 1.]));
    }

    #[test]
    fn can_add_with_refs() {
        let left = make(&[1., 0., 0.]);
        let right = make(&[0., 1., 0.]);

        assert_eq!(&left + &right, make(&[1., 1., 0.]));
        assert_eq!(&left + make(&[0., 1., 0.]), make(&[1., 1., 0.]));
        assert_eq!(make(&[1., 0., 0.]) + &right, make(&[1., 1., 0.]));
    }

    #[test]
    fn can_sub_costs() {
        assert_eq!(make(&[1., 0., 0.]) - make(&[1., 0., 0.]), make(&[0., 0., 0.]));
        assert_eq!(make(&[1., 0., 0.]) - make(&[0., 1., 0.]), make(&[1., -1., 0.]));

        assert_eq!(make(&[1., 0.]) - make(&[0., 1., 0.]), make(&[1., -1., 0.]));
        assert_eq!(make(&[0., 0., 1.]) - make(&[1., 0.]), make(&[-1., 0., 1.]));
    }

    #[test]
    fn can_sub_defaults() {
        assert_eq!(InsertionCost::default() - make(&[0., 1.]), make(&[0., -1.]));
        assert_eq!(make(&[0., 1.]) - InsertionCost::default(), make(&[0., 1.]));
    }

    #[test]
    fn can_sub_with_refs() {
        let left = make(&[1., 0., 0.]);
        let right = make(&[0., 1., 0.]);

        assert_eq!(&left - &right, make(&[1., -1., 0.]));
        assert_eq!(&left - make(&[0., 1., 0.]), make(&[1., -1., 0.]));
        assert_eq!(make(&[1., 0., 0.]) - &right, make(&[1., -1., 0.]));
    }
}

mod debug {
    use super::*;
    use crate::helpers::models::problem::SingleBuilder;
    use crate::helpers::models::solution::{create_empty_route_ctx, test_activity};

    #[test]
    fn can_use_debug_fmt_for_insertion_cost() {
        let cost = InsertionCost::new(&[1., 2., 3.]);

        let result = format!("{cost:?}");

        assert_eq!(result, "[1.0, 2.0, 3.0]")
    }

    #[test]
    fn can_use_debug_fmt_for_insertion_result_with_failure() {
        let result = InsertionResult::make_failure();

        let result = format!("{result:?}");

        assert!(!result.contains("::"));
        assert!(result.contains("constraint: -1"));
        assert!(result.contains("stopped: false"));
        assert!(result.contains("job: None"));
    }

    #[test]
    fn can_use_debug_fmt_for_insertion_result_with_success() {
        let result = InsertionResult::make_success(
            InsertionCost::new(&[1., 2., 3.]),
            SingleBuilder::default().build_as_job_ref(),
            vec![(test_activity(), 1)],
            &create_empty_route_ctx(),
        );

        let result = format!("{result:?}");

        assert!(!result.contains("::"));
        assert!(result.contains("cost"));
        assert!(result.contains("activities"));
        assert!(result.contains("actor"));
    }
}
