// See https://stackoverflow.com/questions/34662713/how-can-i-create-parameterized-tests-in-rust
macro_rules! with_dollar_sign {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
    }
}

/// A macro to a create parameterized test.
#[macro_export]
macro_rules! parameterized_test {
    ($name:ident, $args:pat, $body:tt) => {
        with_dollar_sign! {
        ($d:tt) => {
            macro_rules! $name {
                ($d($d pname:ident: $d values:expr,)*) => {
                    mod $name {
                        use super::*;
                        $d(
                            #[test]
                            fn $d pname() {
                                let $args = $d values;
                                $body
                            }
                        )*
                    }}}}}
    };
}
