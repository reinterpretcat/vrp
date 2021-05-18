use crate::cli::*;

#[test]
fn can_get_app_name() {
    let app = get_app();

    assert_eq!(app.get_name(), "Vehicle Routing Problem Solver");
}
