use super::*;
use crate::construction::heuristics::ActivityContext;
use crate::helpers::models::solution::ActivityBuilder;

fn tier(tier: usize) -> usize {
    tier
}

fn loc(idx: usize) -> Location {
    idx
}

fn simple(id: usize) -> LocationDetail {
    LocationDetail::Simple(id)
}

fn compound(ids: &[usize]) -> LocationDetail {
    LocationDetail::Compound(ids.iter().copied().collect())
}

mod hierarchy {
    use super::*;

    enum Estimate {
        Tier(usize),
        Penalty,
    }

    parameterized_test! {can_estimate_activity, test_data, {
        let mut index = HierarchyIndex::new(3);
        let (data, expected) = test_data;
        assert_eq!(data.len(), 3);

        for (location, loc_data) in data.into_iter().enumerate() {
            for (tier, detail) in loc_data.into_iter() {
                index.insert(location, tier, detail).expect("cannot insert test data");
            }
        }
        let insertion_idx = 0;

        let expected = match expected {
            Estimate::Tier(tier) => index.tiers.get_value(tier).expect("cannot get tier value"),
            Estimate::Penalty => index.tiers.max_penalty_value(),
        };

        can_estimate_activity_impl(index, insertion_idx, expected);
    }}

    can_estimate_activity! {
        case01_tier_zero_all_three_with_same_tier: (vec![
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
        ], Estimate::Tier(0)),

        case02_tier_zero_new_cluster_split: (vec![
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(2))],
        ], Estimate::Penalty),

        case03_tier_zero_same: (vec![
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
        ], Estimate::Tier(0)),

        case04_tier_one_new_with_different_id: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(3))],
          vec![(tier(0), simple(1)), (tier(1), simple(3))],
          vec![(tier(0), simple(2)), (tier(1), simple(3))],
        ], Estimate::Tier(1)),

        case05_tier_one_share_same_id: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(1))],
          vec![(tier(0), simple(1)), (tier(1), simple(1))],
          vec![(tier(0), simple(2)), (tier(1), simple(1))],
        ], Estimate::Tier(1)),

        case06_tier_zero_new_as_border_cluster: (vec![
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(2))],
          vec![(tier(0), simple(2))],
        ], Estimate::Tier(0)),

        case07_tier_one_new_as_border_cluster: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(4))],
          vec![(tier(0), simple(2)), (tier(1), simple(5))],
          vec![(tier(0), simple(3)), (tier(1), simple(4))],
        ], Estimate::Tier(1)),

        case08_tier_one_new_as_border_cluster: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(4))],
          vec![(tier(0), simple(2)), (tier(1), simple(5))],
          vec![(tier(0), simple(3)), (tier(1), simple(5))],
        ], Estimate::Tier(1)),

        case09_tier_two_new_all_different: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(4)), (tier(2), simple(7))],
          vec![(tier(0), simple(2)), (tier(1), simple(5)), (tier(2), simple(8))],
          vec![(tier(0), simple(3)), (tier(1), simple(6)), (tier(2), simple(9))],
        ], Estimate::Penalty),

        case09_tier_two_new_split_cluster: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(0), simple(2)), (tier(1), simple(4)), (tier(2), simple(6))],
        ], Estimate::Penalty),

        case10_tier_two_new_join_cluster_on_third_tier: (vec![
          vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(0), simple(1)), (tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(0), simple(2)), (tier(1), simple(4)), (tier(2), simple(5))],
        ], Estimate::Tier(2)),

        case11_tier_two_new_join_cluster_with_skipping_middle_tier: (vec![
          vec![(tier(0), simple(1)), (tier(2), simple(3))],
          vec![(tier(0), simple(1)), (tier(2), simple(3))],
          vec![(tier(0), simple(2)), (tier(2), simple(3))],
        ], Estimate::Tier(2)),

        case12_tier_two_new_join_cluster_skipping_middle_tier: (vec![
          vec![(tier(0), simple(1)), (tier(2), simple(3))],
          vec![(tier(0), simple(1)), (tier(2), simple(3))],
          vec![(tier(0), simple(2)), (tier(2), simple(4))],
        ], Estimate::Penalty),

        case13_tier_two_new_join_cluster_skipping_zero_tier: (vec![
          vec![(tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(1), simple(3)), (tier(2), simple(5))],
          vec![(tier(1), simple(4)), (tier(2), simple(5))],
        ], Estimate::Tier(2)),

        case14_all_empty: (vec![
          vec![],
          vec![],
          vec![],
        ], Estimate::Penalty),

        case15_new_empty: (vec![
          vec![(tier(0), simple(1))],
          vec![(tier(0), simple(1))],
          vec![],
        ], Estimate::Penalty),

        case16_old_empty: (vec![
          vec![(tier(0), simple(1))],
          vec![],
          vec![],
        ], Estimate::Penalty),

        case16_compound_intersection: (vec![
          vec![(tier(0), compound(&[1, 2]))],
          vec![(tier(0), compound(&[2, 3]))],
          vec![(tier(0), compound(&[2]))],
        ], Estimate::Tier(0)),

        case17_compound_disjoint: (vec![
          vec![(tier(0), compound(&[1, 2]))],
          vec![(tier(0), compound(&[3, 4]))],
          vec![(tier(0), compound(&[5, 6]))],
        ], Estimate::Penalty),
    }

    fn can_estimate_activity_impl(hierarchy_index: HierarchyIndex, insertion_idx: usize, expected: usize) {
        let next_activity = ActivityBuilder::with_location(1).build();
        let activity_ctx = ActivityContext {
            index: insertion_idx,
            prev: &ActivityBuilder::with_location(0).build(),
            target: &ActivityBuilder::with_location(2).build(),
            next: Some(&next_activity),
        };

        let result = get_activity_penalty(&activity_ctx, &hierarchy_index);

        assert_eq!(result, expected);
    }

    #[test]
    fn can_generate_expected_tier_values() {
        assert_eq!(vec![0, 1, 3], Tiers::new(3).iter().map(|tier| tier.value()).collect::<Vec<_>>());

        assert_eq!(vec![0, 1, 3, 7, 15], Tiers::new(5).iter().map(|tier| tier.value()).collect::<Vec<_>>());

        assert_eq!(vec![0, 1, 3, 7, 15, 31, 63], Tiers::new(7).iter().map(|tier| tier.value()).collect::<Vec<_>>());
    }

    #[test]
    fn can_create_hierarchy_index_from_clusters() {
        let hierarchy = vec![
            HashMap::from([(2, vec![0, 1, 2, 3, 4]), (7, vec![5, 6, 7, 8, 9])]),
            HashMap::from([(1, vec![0, 1, 2]), (3, vec![3, 4]), (6, vec![5, 6]), (7, vec![7, 8, 9])]),
            HashMap::from([
                (0, vec![0, 1]),
                (2, vec![2]),
                (3, vec![3]),
                (4, vec![4]),
                (5, vec![5]),
                (6, vec![6]),
                (7, vec![7, 8, 9]),
            ]),
        ];
        let tiers = hierarchy.len();

        let hierarchy_index = HierarchyIndex::try_from(&hierarchy).expect("cannot create hierarchy index");

        assert_eq!(hierarchy_index.tiers.0.len(), tiers);
        let assert_fn = |location: Location, expected: Vec<(usize, Vec<Location>)>| {
            let actual = hierarchy_index.get(&location).expect("no location in index");
            let actual = (0..tiers)
                .map(|tier_idx| {
                    let tier = hierarchy_index.tiers.get(tier_idx).expect("cannot get tier");
                    let detail_id = *actual.get(tier).expect("cannot get detail").as_simple().expect("must be simple");

                    match tier_idx {
                        0 => assert_eq!(tier.value(), 0),
                        1 => assert_eq!(tier.value(), 1),
                        2 => assert_eq!(tier.value(), 3),
                        _ => unreachable!("unexpected tier value"),
                    }

                    let mut locations = hierarchy_index
                        .index
                        .iter()
                        //.filter(|(other_location, _)| **other_location != location)
                        .filter_map(|(location, details)| {
                            details
                                .get(tier)
                                .and_then(|details| details.as_simple().copied())
                                .filter(|&detail| detail == detail_id)
                                .map(|_| *location)
                        })
                        .collect::<Vec<_>>();
                    locations.sort();

                    (tier_idx, locations)
                })
                .collect::<Vec<_>>();

            assert_eq!(actual, expected);
        };
        assert_fn(0, vec![(0, vec![0, 1]), (1, vec![0, 1, 2]), (2, vec![0, 1, 2, 3, 4])]);
        assert_fn(1, vec![(0, vec![0, 1]), (1, vec![0, 1, 2]), (2, vec![0, 1, 2, 3, 4])]);
        assert_fn(2, vec![(0, vec![2]), (1, vec![0, 1, 2]), (2, vec![0, 1, 2, 3, 4])]);

        assert_fn(3, vec![(0, vec![3]), (1, vec![3, 4]), (2, vec![0, 1, 2, 3, 4])]);
        assert_fn(4, vec![(0, vec![4]), (1, vec![3, 4]), (2, vec![0, 1, 2, 3, 4])]);

        assert_fn(5, vec![(0, vec![5]), (1, vec![5, 6]), (2, vec![5, 6, 7, 8, 9])]);
        assert_fn(6, vec![(0, vec![6]), (1, vec![5, 6]), (2, vec![5, 6, 7, 8, 9])]);

        assert_fn(7, vec![(0, vec![7, 8, 9]), (1, vec![7, 8, 9]), (2, vec![5, 6, 7, 8, 9])]);
        assert_fn(8, vec![(0, vec![7, 8, 9]), (1, vec![7, 8, 9]), (2, vec![5, 6, 7, 8, 9])]);
        assert_fn(9, vec![(0, vec![7, 8, 9]), (1, vec![7, 8, 9]), (2, vec![5, 6, 7, 8, 9])]);
    }

    #[test]
    fn can_handle_empty_hierarchy() {
        let index = HierarchyIndex::try_from(&vec![]);

        assert!(index.is_err());
    }

    #[test]
    fn can_handle_hierarchy_index_insertion_with_invalid_index() {
        let mut index =
            HierarchyIndex::try_from(&vec![HashMap::from([(2, vec![0, 1, 2, 3, 4]), (7, vec![5, 6, 7, 8, 9])])])
                .unwrap();

        let result = index.insert(1, 2, LocationDetail::new_simple(0));

        assert!(result.is_err());
    }
}

mod estimations {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::problem::FleetBuilder;
    use crate::helpers::models::solution::{RouteBuilder, RouteContextBuilder};
    use crate::models::common::Distance;
    use crate::models::problem::Driver;
    use crate::models::solution::Activity;

    struct DistanceObjective {
        matrix: Vec<Distance>,
        size: usize,
    }

    impl FeatureObjective for DistanceObjective {
        fn fitness(&self, solution: &InsertionContext) -> Cost {
            unreachable!()
        }

        fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
            // simply calculate distance delta
            match move_ctx {
                MoveContext::Route { .. } => Cost::default(),
                MoveContext::Activity { activity_ctx, .. } => {
                    let prev = activity_ctx.prev.place.location;
                    let target = activity_ctx.target.place.location;
                    if let Some(next) = activity_ctx.next.map(|n| n.place.location) {
                        let prev_target = self.matrix[prev * self.size + target];
                        let target_next = self.matrix[target * self.size + next];
                        let prev_next = self.matrix[prev * self.size + next];

                        target_next + prev_target - prev_next
                    } else {
                        self.matrix[prev * self.size + target]
                    }
                }
            }
        }
    }

    fn activity(location: &Location) -> Activity {
        ActivityBuilder::with_location(*location).build()
    }

    /// Creates the main feature with three levels of predefined clusters,
    /// 13 locations in total (0 for depot, 1-12 for jobs).
    fn create_test_feature() -> Feature {
        #[rustfmt::skip]
        let points = [
            (0, 0),
            (-10,  5), (-5,  5), (5,  5), (10,  5),   //    1   2      3  4
            (-10,  0), (-5,  0), (5,  0), (10,  0),   //    5   6   0  7  8
            (-10, -5), (-5, -5), (5, -5), (10, -5),   //    9  10      11 12
        ];
        #[rustfmt::skip]
        let clusters = vec![
            HashMap::from([(6, vec![0, 1, 2, 5, 6, 9, 10]), (8, vec![3, 4, 7, 8, 11, 12])]),
            HashMap::from([(1, vec![1, 2, 5]), (10, vec![0, 6, 9, 10]), (3, vec![3, 4, 7]), (12, vec![8, 11, 12])]),
            HashMap::from([
                (1, vec![1, 2]), (5, vec![5]), (6, vec![0, 6]), (10, vec![9, 10]),
                (3, vec![3, 4]), (7, vec![7]), (8, vec![8]),  (12, vec![11, 12])]),
        ];
        let size = points.len();
        let matrix = (0..size)
            .flat_map(|from| (0..size).map(move |to| (from, to)))
            .map(|(from, to)| {
                let ((from_x, from_y), (to_x, to_y)) = (points[from], points[to]);
                let (dx, dy) = ((from_x - to_x) as Distance, (from_y - to_y) as Distance);
                (dx * dx + dy * dy).sqrt()
            })
            .collect::<Vec<_>>();

        let distance_feature = FeatureBuilder::default()
            .with_name("distance")
            .with_objective(DistanceObjective { matrix: matrix.clone(), size })
            .build()
            .unwrap();

        create_hierarchical_areas_feature(distance_feature, &clusters, move |_, from, to| matrix[from * size + to])
            .unwrap()
    }

    fn create_solution_ctx(routes: &[Vec<Location>]) -> SolutionContext {
        let fleet = FleetBuilder::default()
            .add_driver(Driver::empty())
            .add_vehicles(
                (0..routes.len())
                    .map(|idx| {
                        VehicleBuilder::default()
                            .id(&format!("v{idx}"))
                            .add_detail(
                                VehicleDetailBuilder::default()
                                    .set_start_location(0)
                                    .set_end_location(0)
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap()
                    })
                    .collect(),
            )
            .build();

        TestInsertionContextBuilder::default()
            .with_routes(
                routes
                    .iter()
                    .enumerate()
                    .map(|(idx, locations)| {
                        RouteContextBuilder::default()
                            .with_route(
                                RouteBuilder::default()
                                    .with_vehicle(&fleet, &format!("v{idx}"))
                                    .add_activities(locations.iter().map(activity))
                                    .build(),
                            )
                            .build()
                    })
                    .collect(),
            )
            .build()
            .solution
    }

    fn create_tier_index(clusters: &[Vec<Location>]) -> HashMap<Tier, HashSet<Location>> {
        Tiers::new(clusters.len())
            .iter()
            .zip(clusters)
            .map(|(tier, medoids)| (tier.clone(), HashSet::<Location>::from_iter(medoids.iter().copied())))
            .collect()
    }

    #[test]
    fn can_accept_insertion() {
        let state = create_test_feature().state.unwrap();
        let mut solution_ctx = create_solution_ctx(&[vec![6, 5, 2]]);
        state.accept_route_state(&mut solution_ctx.routes[0]);

        state.accept_insertion(
            &mut solution_ctx,
            0,
            &SingleBuilder::default().location(8).unwrap().build_as_job().unwrap(),
        );

        let medoid_index = solution_ctx.routes[0].state().get_medoid_index().expect("no medoid index");
        assert_eq!(*medoid_index, create_tier_index(&[vec![1, 5, 6, 8], vec![1, 10, 12], vec![6, 8]]))
    }

    #[test]
    fn can_accept_solution_state() {
        let state = create_test_feature().state.unwrap();
        let mut solution_ctx = create_solution_ctx(&[vec![6, 5, 2]]);

        state.accept_solution_state(&mut solution_ctx);

        let medoid_index = solution_ctx.routes[0].state().get_medoid_index().expect("no medoid index");
        assert_eq!(*medoid_index, create_tier_index(&[vec![1, 5, 6], vec![1, 10], vec![6]]))
    }

    parameterized_test! {can_estimate_job, (routes, selected_route_idx, new_job_location, expected_cost), {
        can_estimate_job_impl(routes, selected_route_idx, new_job_location, expected_cost).unwrap();
    }}

    can_estimate_job! {
        case01: (vec![vec![1, 2, 5], vec![9, 10], vec![3, 4]], 2, 6, 777.),
    }

    fn can_estimate_job_impl(
        routes: Vec<Vec<Location>>,
        selected_route_idx: usize,
        new_job_location: usize,
        expected_cost: Cost,
    ) -> GenericResult<()> {
        let feature = create_test_feature();
        let (objective, state) = (feature.objective.unwrap(), feature.state.unwrap());
        let mut solution_ctx = create_solution_ctx(&routes);
        state.accept_solution_state(&mut solution_ctx);

        let estimate = objective.estimate(&MoveContext::Route {
            solution_ctx: &solution_ctx,
            route_ctx: &solution_ctx.routes[selected_route_idx],
            job: &SingleBuilder::default().location(new_job_location)?.build_as_job()?,
        });

        assert_eq!(estimate, expected_cost);

        Ok(())
    }
}
