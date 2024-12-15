use super::*;
use crate::models::common::{Distance, Duration};
use crate::models::problem::TravelTime;
use crate::models::solution::Route;
use rosomaxa::prelude::DefaultRandom;
use std::collections::HashSet;

struct MockTransportCost;
impl TransportCost for MockTransportCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        todo!()
    }

    fn distance_approx(&self, _profile: &Profile, from: Location, to: Location) -> f64 {
        (from as f64 - to as f64).abs()
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        todo!()
    }

    fn distance(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Distance {
        todo!()
    }

    fn size(&self) -> usize {
        100
    }
}

#[test]
fn can_create_multi_tier_clusters() -> GenericResult<()> {
    let profile = Profile::default();
    let transport = MockTransportCost;
    let random = DefaultRandom::new_repeatable();
    let expected_cluster_nums = [2, 3, 4, 5, 8, 10, 12, 16, 32];

    let clusters = create_multi_tier_clusters(profile, &transport, &random)?;

    assert_eq!(clusters.len(), expected_cluster_nums.len());
    clusters.iter().zip(expected_cluster_nums.iter()).for_each(|(clusters, expected_num)| {
        assert_eq!(clusters.len(), *expected_num);
        let total = clusters.iter().flat_map(|(_, cluster)| cluster.iter()).collect::<HashSet<_>>().len();
        assert_eq!(total, transport.size());
    });

    Ok(())
}
