use super::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_index_with_coordinate_an_unknown_location_types() {
    let unknown_location = Location::Custom { r#type: CustomLocationType::Unknown };
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                Job {
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: unknown_location.clone(),
                            duration: 0.,
                            times: None,
                            tag: None,
                        }],
                        demand: None,
                        order: None,
                    }]),
                    ..create_job("job3")
                },
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };

    let index = CoordIndex::new(&problem);

    assert!(index.has_coordinates());
    assert!(index.has_custom());
    assert!(!index.has_indices());
    assert_eq!(index.max_matrix_index(), 2);
    assert_eq!(index.custom_locations_len(), 1);
    // Location::Coordinate type
    assert_eq!(index.get_by_loc(&(1., 0.).to_loc()), Some(0));
    assert_eq!(index.get_by_loc(&(2., 0.).to_loc()), Some(1));
    assert_eq!(index.get_by_loc(&(0., 0.).to_loc()), Some(2));
    assert_eq!(index.get_by_idx(0), Some((1., 0.).to_loc()));
    assert_eq!(index.get_by_idx(1), Some((2., 0.).to_loc()));
    assert_eq!(index.get_by_idx(2), Some((0., 0.).to_loc()));
    assert!(!index.is_special_index(0));
    assert!(!index.is_special_index(1));
    assert!(!index.is_special_index(2));
    // Location::Custom
    assert_eq!(index.get_by_loc(&unknown_location), Some(9));
    assert_eq!(index.get_by_idx(9), Some(unknown_location));
    assert!(index.is_special_index(9));
    // out of range
    assert_eq!(index.get_by_loc(&(3., 0.).to_loc()), None);
    assert_eq!(index.get_by_idx(3), None);
    assert_eq!(index.get_by_idx(8), None);
    assert_eq!(index.get_by_idx(10), None);
    assert!(!index.is_special_index(3));
}
