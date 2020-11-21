use crate::algorithms::gsom::get_network_state;
use crate::helpers::algorithms::gsom::create_test_network;

#[test]
fn can_get_state() {
    let network = create_test_network();

    let state = get_network_state(&network);

    assert_eq!(state.nodes.len(), 4);
    assert_eq!(state.shape, (0..1, 0..1, 3));
}

#[test]
fn can_format_state() {
    let network = create_test_network();
    let state = get_network_state(&network);

    let result = format!("{}", state);

    assert!(result.starts_with("(0,1,0,1,3,[("));
}
