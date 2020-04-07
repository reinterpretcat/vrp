use super::*;

fn create_matrix_data(
    profile: Profile,
    timestamp: Option<Timestamp>,
    duration: (Duration, usize),
    distance: (Distance, usize),
) -> MatrixData {
    MatrixData { profile, timestamp, durations: vec![duration.0; duration.1], distances: vec![distance.0; distance.1] }
}

mod time_aware {
    use super::*;

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
}
