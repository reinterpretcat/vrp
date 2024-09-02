use super::*;

mod time_window {
    use super::*;
    use crate::models::common::Distance;

    parameterized_test! {can_get_distance, (first, second, expected), {
        can_get_distance_impl(TimeWindow::new(first.0, first.1), TimeWindow::new(second.0, second.1), expected);
    }}

    can_get_distance! {
        case_01: ((0, 10), (8, 12), 0),
        case_02: ((0, 10), (12, 20), 2),
        case_03: ((12, 20), (0, 11), 1),
    }

    fn can_get_distance_impl(first: TimeWindow, second: TimeWindow, expected: Distance) {
        assert_eq!(first.distance(&second), expected);
    }

    parameterized_test! {can_get_overlapping, (first, second, expected), {
        can_get_overlapping_impl(TimeWindow::new(first.0, first.1),
            TimeWindow::new(second.0, second.1), expected.map(|(start, end)| TimeWindow::new(start, end)));
    }}

    can_get_overlapping! {
        case_01: ((0, 10), (8, 12), Some((8, 10))),
        case_02: ((8, 12), (0, 10), Some((8, 10))),
        case_03: ((0, 10), (5, 8), Some((5, 8))),
        case_04: ((0, 10), (10, 12), Some((10, 10))),
        case_05: ((0, 10), (0, 10), Some((0, 10))),
        case_06: ((0, 10), (11, 20), None),
    }

    fn can_get_overlapping_impl(first: TimeWindow, second: TimeWindow, expected: Option<TimeWindow>) {
        assert_eq!(first.overlapping(&second), expected);
    }

    parameterized_test! {can_get_duration, (first, expected), {
        can_get_duration_impl(TimeWindow::new(first.0, first.1), expected);
    }}

    can_get_duration! {
        case_01: ((0, 10), 10),
        case_02: ((7, 10), 3),
        case_03: ((10, 10), 0),
    }

    fn can_get_duration_impl(time: TimeWindow, expected: Duration) {
        assert_eq!(time.duration(), expected);
    }
}
