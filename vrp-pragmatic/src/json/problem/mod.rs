//! Specifies logic to read problem and routing matrix from json input.
//!
//! ## Problem format
//!
//! Pragmatic problem format aims to support rich VRP problem which consists of multiple VRP variations.
//!
//! You can construct problem definition using [`Problem`] model or specify it in json:
//!
//! ```json
//! {
//!     "id": "my_problem",
//!     "plan": {
//!         "jobs": [],
//!         "relations": []
//!     },
//!     "fleet": {
//!         "types": [],
//!         "profiles": []
//!     },
//!     "config": {}
//! }
//! ```
//!
//! ## Routing Matrix format
//!
//! Routing matrix can be constructed using [`Matrix`] object or json:
//!
//! ```json
//! {
//!     "numOrigins": 2,
//!     "numDestinations": 2,
//!     "travelTimes": [0,1,1,0],
//!     "distances": [0,2,2,0]
//! }
//! ```
//! where:
//! - `numOrigins` and `numDestinations`: number of unique locations.
//! - `travelTimes` is square matrix of durations in seconds represented via single dimensional array.
//! - `distances` is square matrix of distances in meters represented via single dimensional array.
//!
//!  Both durations and distances are mapped to the list of unique locations generated from the problem
//!  definition. In this list, locations are specified in the order they defined. For example, if you
//!  have two jobs with locations A and B, one vehicle type with depot location C, then you have
//!  the following location list: A,B,C. It corresponds to the matrix (durations or distances):
//!
//! ```md
//! |----|----|----|
//! |  0 | BA | CA |
//! | AB |  0 | CB |
//! | AC | BC |  0 |
//!```
//!
//!  where
//! - `0`: zero duration or distance
//! - `XY`: distance or duration from X location to Y
//!
//!  As single dimensional array it looks like:
//!
//! `[0,BA,CA,AB,0,CB,AC,BC,0]`
//!
//!  Check `create_coord_index` function for more details.
//!
//!For complete examples, please check `data` folder and `features` tests in the source code repository.

mod deserializer;
pub use self::deserializer::*;

mod reader;
pub use self::reader::PragmaticProblem;
