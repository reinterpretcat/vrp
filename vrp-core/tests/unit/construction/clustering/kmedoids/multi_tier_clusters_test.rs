use super::*;

#[test]
fn can_add_observations() {
    let mut remedians = DistanceRemedians::default();
    let distances = (0..100).enumerate().map(|(idx, d)| (idx, d as Distance)).collect::<Vec<_>>();

    distances.iter().for_each(|(_, d)| remedians.add_main_observation(*d));
    remedians.add_extra_observations(&distances);

    assert_eq!(remedians.remedians[0].approx_median().unwrap(), 49.);
    assert_eq!(remedians.remedians[1].approx_median().unwrap(), 4.);
    assert_eq!(remedians.remedians[2].approx_median().unwrap(), 8.);
    assert_eq!(remedians.remedians[3].approx_median().unwrap(), 16.);
}

#[test]
fn can_get_averages_of_remedians() {
    let mut remedians1 = DistanceRemedians::default();
    let mut remedians2 = DistanceRemedians::default();

    let distances1 =
        (0..100).enumerate().filter(|(idx, _)| idx % 2 == 0).map(|(idx, d)| (idx, d as Distance)).collect::<Vec<_>>();
    let distances2 =
        (0..100).enumerate().filter(|(idx, _)| idx % 2 == 1).map(|(idx, d)| (idx, d as Distance)).collect::<Vec<_>>();

    distances1.iter().for_each(|(_, d)| remedians1.add_main_observation(*d));
    remedians1.add_extra_observations(&distances1.into_iter().collect::<Vec<_>>());
    distances2.iter().for_each(|(_, d)| remedians2.add_main_observation(*d));
    remedians2.add_extra_observations(&distances2.into_iter().collect::<Vec<_>>());

    let mut avg_distances = EpsilonDistances::default();

    avg_distances.add(&remedians1);
    avg_distances.add(&remedians2);

    let avg_distances = avg_distances.into_iter().collect::<Vec<_>>();

    assert_eq!(avg_distances[0], 8.5);
    assert_eq!(avg_distances[1], 16.5);
    assert_eq!(avg_distances[2], 32.5);
    assert_eq!(avg_distances[3], 54.5);
}
