use super::*;

fn create_matrix_data(
    profile: Profile,
    timestamp: Option<Timestamp>,
    duration: (Duration, usize),
    distance: (Distance, usize),
) -> MatrixData {
    MatrixData { profile, timestamp, durations: vec![duration.0; duration.1], distances: vec![distance.0; distance.1] }
}

#[test]
fn can_detect_dimensions_mismatch() {
    assert_eq!(
        create_matrix_transport_cost(vec![
            create_matrix_data(0, Some(0.), (0., 2), (0., 2)),
            create_matrix_data(0, Some(1.), (0., 1), (0., 2)),
        ])
        .err(),
        Some("Distance and duration collections have different length".to_string())
    );
}

#[test]
fn can_return_error_when_mixing_timestamps() {
    assert_eq!(
        TimeAwareMatrixTransportCost::new(vec![create_matrix_data(0, None, (0., 1), (0., 1))], 1).err(),
        Some("Cannot use matrix without timestamp".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(
            vec![create_matrix_data(0, Some(0.), (0., 1), (0., 1)), create_matrix_data(0, None, (0., 1), (0., 1))],
            1,
        )
        .err(),
        Some("Cannot use matrix without timestamp".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(vec![create_matrix_data(0, Some(0.), (0., 1), (0., 1))], 1).err(),
        Some("Should not use time aware matrix routing with single matrix".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(
            vec![
                create_matrix_data(0, Some(0.), (1., 1), (1., 1)), //
                create_matrix_data(0, Some(1.), (1., 1), (1., 1)), //
                create_matrix_data(1, Some(0.), (1., 1), (1., 1)), //
            ],
            1,
        )
        .err(),
        Some("Should not use time aware matrix routing with single matrix".to_string())
    );
}

#[test]
fn can_interpolate_durations() {
    let costs = TimeAwareMatrixTransportCost::new(
        vec![
            create_matrix_data(0, Some(0.), (100., 2), (1., 2)),
            create_matrix_data(0, Some(10.), (200., 2), (1., 2)),
            create_matrix_data(1, Some(0.), (300., 2), (5., 2)),
            create_matrix_data(1, Some(10.), (400., 2), (5., 2)),
        ],
        2,
    )
    .unwrap();

    for (timestamp, duration) in vec![(0., 100.), (10., 200.), (15., 200.), (3., 130.), (5., 150.), (7., 170.)] {
        assert_eq!(costs.duration(0, 0, 1, timestamp), duration);
    }

    for (timestamp, duration) in vec![(0., 300.), (10., 400.), (15., 400.), (3., 330.), (5., 350.), (7., 370.)] {
        assert_eq!(costs.duration(1, 0, 1, timestamp), duration);
    }

    assert_eq!(costs.distance(0, 0, 1, 0.), 1.);
    assert_eq!(costs.distance(1, 0, 1, 0.), 5.);
}
