mod comparison;

pub use self::comparison::compare_floats;
pub use self::comparison::compare_shared;

mod random;
pub use self::random::DefaultRandom;
pub use self::random::Random;

mod routing;
pub use self::routing::MatrixFactory;

// See https://stackoverflow.com/questions/38863781/how-to-create-a-tuple-from-a-vector
pub trait TryCollect<T> {
    fn try_collect(&mut self) -> Option<T>;
}

#[macro_export]
macro_rules! impl_try_collect_tuple {
    () => { };
    ($A:ident $($I:ident)*) => {
        impl_try_collect_tuple!($($I)*);

        impl<$A: Iterator> TryCollect<($A::Item, $($I::Item),*)> for $A {
            fn try_collect(&mut self) -> Option<($A::Item, $($I::Item),*)> {
                let r = (try_opt!(self.next()),
                         // hack: we need to use $I in the expansion
                         $({ let a: $I::Item = try_opt!(self.next()); a}),* );
                Some(r)
            }
        }
    }
}

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
