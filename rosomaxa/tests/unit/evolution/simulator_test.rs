use super::*;
use crate::utils::DefaultRandom;

#[test]
fn can_select_initial_operators_in_order_before_using_repeat_pool() {
    let random = DefaultRandom::new_repeatable();

    assert_eq!(select_initial_operator_index(0, 3, &[1], &[1], &random), Some(0));
    assert_eq!(select_initial_operator_index(1, 3, &[1], &[1], &random), Some(1));
    assert_eq!(select_initial_operator_index(2, 3, &[1], &[1], &random), Some(2));
}

#[test]
fn can_exclude_one_shot_initial_operator_from_repeated_selection() {
    let random = DefaultRandom::new_repeatable();

    assert_eq!(select_initial_operator_index(3, 3, &[1], &[1], &random), Some(1));
}

#[test]
fn can_stop_when_no_initial_operator_is_repeatable() {
    let random = DefaultRandom::new_repeatable();

    assert_eq!(select_initial_operator_index(3, 3, &[], &[], &random), None);
}
