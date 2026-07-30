use super::*;

#[test]
fn can_vary_number_of_skipped_routes() {
    let route_count = 10;

    let skips = (0..=4).map(|sample| get_route_skip(sample, route_count)).collect::<Vec<_>>();

    assert_eq!(skips, vec![0, 1, 2, 3, 4]);
}

#[test]
fn can_keep_at_least_one_existing_route() {
    assert_eq!(get_route_skip(4, 0), 0);
    assert_eq!(get_route_skip(4, 1), 0);
    assert_eq!(get_route_skip(4, 2), 1);
    assert_eq!(get_route_skip(4, 3), 2);
}
