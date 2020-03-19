#[cfg(test)]
#[path = "../../tests/unit/extensions/approximated_transport_cost_test.rs"]
mod approximated_transport_cost_test;

use crate::json::Location;
use vrp_core::models::problem::{MatrixTransportCost, TransportCost};

/// An implementation of `TransportCost` which uses approximation to get distance and duration
/// between geo locations.
pub struct ApproximatedTransportCost {
    matrix_costs: MatrixTransportCost,
}

impl ApproximatedTransportCost {
    /// Creates a new instance of `ApproximatedTransportCost` using `locations`
    /// and fixed speed (in meters per second).
    pub fn new(locations: &Vec<Location>, speed: f64) -> Self {
        let (distances, durations): (Vec<_>, Vec<_>) = locations
            .iter()
            .flat_map(|l1| {
                locations.iter().map(move |l2| {
                    let distance = get_distance(l1, l2);
                    let duration = distance / speed;
                    (distance, duration)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .unzip();

        Self { matrix_costs: MatrixTransportCost::new(vec![durations], vec![distances]) }
    }
}

impl TransportCost for ApproximatedTransportCost {
    fn duration(&self, profile: i32, from: usize, to: usize, departure: f64) -> f64 {
        self.matrix_costs.duration(profile, from, to, departure)
    }

    fn distance(&self, profile: i32, from: usize, to: usize, departure: f64) -> f64 {
        self.matrix_costs.distance(profile, from, to, departure)
    }
}

/// Gets distance between two points using haversine formula.
fn get_distance(p1: &Location, p2: &Location) -> f64 {
    let d_lat = degree_rad(p1.lat - p2.lat);
    let d_lng = degree_rad(p1.lng - p2.lng);

    let lat1 = degree_rad(p1.lat);
    let lat2 = degree_rad(p2.lat);

    let a =
        (d_lat / 2.).sin() * (d_lat / 2.).sin() + (d_lng / 2.).sin() * (d_lng / 2.).sin() * (lat1).cos() * (lat2).cos();
    let c = 2. * a.sqrt().atan2((1. - a).sqrt());

    let radius = wgs84_earth_radius(d_lat);

    radius * c
}

/// Converts degrees to radians.
#[inline(always)]
fn degree_rad(degrees: f64) -> f64 {
    std::f64::consts::PI * degrees / 180.
}

#[inline(always)]
fn wgs84_earth_radius(lat: f64) -> f64 {
    // Semi-axes of WGS-84 geoidal reference
    const WGS84_A: f64 = 6378137.0; // Major semiaxis [m]
    const WGS84_B: f64 = 6356752.3; // Minor semiaxis [m]

    // http://en.wikipedia.org/wiki/Earth_radius
    let an = WGS84_A * WGS84_A * lat.cos();
    let bn = WGS84_B * WGS84_B * lat.sin();
    let ad = WGS84_A * lat.cos();
    let bd = WGS84_B * lat.sin();

    ((an * an + bn * bn) / (ad * ad + bd * bd)).sqrt()
}
