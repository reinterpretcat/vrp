//! Contains common text reading and writing functionality.

mod text_reader;

pub(crate) use self::text_reader::*;
use std::sync::Arc;

mod text_writer;
pub(crate) use self::text_writer::*;

mod initial_reader;
pub use self::initial_reader::read_init_solution;

mod routing;

pub use self::routing::{CoordIndex, CoordIndexExtras};

use vrp_core::models::{Extras, ExtrasBuilder};
use vrp_core::solver::HeuristicFilterExtras;

pub(crate) fn get_extras(coord_index: CoordIndex) -> Extras {
    let mut extras = ExtrasBuilder::default().build().expect("cannot build extras");

    extras.set_coord_index(coord_index);
    extras.set_heuristic_filter(Arc::new(|name| name != "local_reschedule_departure"));

    extras
}

/// A trait to get tuple from collection items.
/// See `<https://stackoverflow.com/questions/38863781/how-to-create-a-tuple-from-a-vector>`
pub(crate) trait TryCollect<T> {
    fn try_collect_tuple(&mut self) -> Option<T>;
}

/// A macro to get tuple from collection items.
#[macro_export]
macro_rules! impl_try_collect_tuple {
    () => { };
    ($A:ident $($I:ident)*) => {
        impl_try_collect_tuple!($($I)*);

        impl<$A: Iterator> TryCollect<($A::Item, $($I::Item),*)> for $A {
            fn try_collect_tuple(&mut self) -> Option<($A::Item, $($I::Item),*)> {
                let r = (try_opt!(self.next()),
                         // hack: we need to use $I in the expansion
                         $({ let a: $I::Item = try_opt!(self.next()); a}),* );
                Some(r)
            }
        }
    }
}

/// A helper macro for getting tuple of collection items.
#[macro_export]
macro_rules! try_opt {
    ($e:expr) => {
        match $e {
            Some(e) => e,
            None => return None,
        }
    };
}

// implement TryCollect<T> where T is a tuple with size 1, 2, .., 10
impl_try_collect_tuple!(A A A A A A A A A A);
