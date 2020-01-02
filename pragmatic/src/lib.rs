//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.
//!
//!
//! ## Supported VRP variations
//!
//! This list tries to classify supported VRP variations using common terminology and notations.
//!
//! - **Capacitated VRP (CVRP)**: designs optimal delivery routes where each vehicle only travels
//!     one route, each vehicle has the same characteristics and there is only one central depot.
//!
//! - **Heterogeneous Fleet VRP (HFVRP)** aka Mixed Fleet VRP: extend CVRP problem by varying the capacities.
//!
//! - **VRP with Time Windows (VRPTW)**: assumes that deliveries to a given customer must occur in a
//!     certain time interval, which varies from customer to customer.
//!
//! - **VRP with Pickup and Delivery (VRPPD)**: goods need to be picked up from a certain location and
//!     dropped off at their destination. The pick-up and drop-off must be done by the same vehicle,
//!     which is why the pick-up location and drop-off location must be included in the same route.
//!
//! - **VRP with backhauls (VRPB)**: a vehicle does deliveries as well as pick-ups in one route.
//!     Some customers require deliveries (referred to as linehauls) and others require pick-ups
//!     (referred to as backhauls).
//!
//! - **Multi-Depot VRP (MDVRP)**: assumes that multiple depots are geographically spread among
//!     the customers
//!
//! - **Open VRP**: usually, a route beginning at a given depot must finish at this depot, but in
//!     this variation vehicle ends at the last served customer.
//!
//! - **Periodic VRP (PVRP)**: is used when planning is made over a certain period and deliveries
//!     to the customer can be made in different days. In current implementation each customer
//!     is visited only once.
//!
//! - **Traveling Salesman Problem (TSP)**: this is a specific case of VRP when there is only one vehicle.
//!
//! In general, all these variations can be combined together in one single problem definition.
//!
//!
//! ## Featured variations
//!
//! This list describes some of supported features in informal way:
//!
//! - **multiple breaks** with multiple time windows and optional location for vehicles.
//!
//! - **multiple shifts** for vehicles: this allows to define multi-day planning scenario when
//!     vehicle can be used multiple times, but on different days.
//!
//! - **multiple reloads**: this allows vehicle to return back to the depot (or any other place) in
//!     order to unload/load goods during single tour. In some VRP variations this helps to significantly
//!     reduce amount of used vehicles.
//!
//! - **multi jobs**: multi job is a job which consists of multiple sub-jobs. Multi job is considered
//!     as assigned only when all of sub jobs are assigned. This is useful for scenarios such as
//!     multiple pickups, but single delivery, or other way round.
//!
//! - **multiple vehicle profiles**: allows to use different routing matrix for different vehicles.
//!     This is useful when fleet consists of different vehicle types, such as truck and car.
//!
//!- **skills**: allows to specify various skills (which is simple some tag) on vehicle and customer.
//!     Customer with specific skills can be visited only if these skills are present on vehicle.
//!
//! - **relations**: allows to specify relations which locks jobs to specific vehicles in
//!     customizable way.
//!
//! - **limits**: allows to specify limits on vehicle such as max traveling distance or time.
//!
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

#[cfg(test)]
#[path = "../tests/features/mod.rs"]
pub mod features;

mod constraints;
mod extensions;
mod utils;

pub mod json;

use crate::json::problem::PragmaticProblem;
use crate::json::solution::PragmaticSolution;
use solver::SolverBuilder;
use std::ffi::{CStr, CString};
use std::io::BufWriter;
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::slice;
use std::sync::Arc;

// TODO improve error propagation

type Callback = extern "C" fn(*const c_char);

fn to_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    std::str::from_utf8(slice).unwrap().to_string()
}

#[no_mangle]
extern "C" fn solve(
    problem: *const c_char,
    matrices: *const *const c_char,
    matrices_len: *const i32,
    success: Callback,
    failure: Callback,
) {
    let result = catch_unwind(|| {
        let problem = to_string(problem);
        let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len as usize).to_vec() };
        let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

        let problem = Arc::new((problem, matrices).read_pragmatic().unwrap());

        let (solution, _, _) = SolverBuilder::default().build().solve(problem.clone()).unwrap();

        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        solution.write_pragmatic(&problem, writer).ok();

        buffer
    });

    match result {
        Ok(solution) => {
            let solution = CString::new(solution.as_bytes()).unwrap();
            success(solution.as_ptr());
        }
        Err(_) => {
            let error = CString::new("Cannot solve".as_bytes()).unwrap();
            failure(error.as_ptr());
        }
    };
}
