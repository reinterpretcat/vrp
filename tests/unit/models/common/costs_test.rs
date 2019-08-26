use super::*;

#[test]
fn objective_cost_actual_returns_total_cost() {
    assert_eq!(
        ObjectiveCost {
            actual: 10.0,
            penalty: 15.0
        }
        .total(),
        25.0
    );
}
