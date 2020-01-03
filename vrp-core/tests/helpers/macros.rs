// See https://stackoverflow.com/questions/34662713/how-can-i-create-parameterized-tests-in-rust
macro_rules! with_dollar_sign {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
    }
}

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

// See https://stackoverflow.com/questions/30429801/whats-the-most-idiomatic-way-to-test-two-options-for-equality-when-they-contain
#[allow(unused)]
macro_rules! cmp_eq_option {
    ($left:expr, $right:expr) => {{
        match (&$left, &$right) {
            (Some(left_val), Some(right_val)) => *left_val == *right_val,
            (None, None) => true,
            _ => false,
        }
    }};
}

#[macro_export]
macro_rules! assert_eq_option {
    ($left:expr, $right:expr) => ({
        if !cmp_eq_option!($left, $right) {
            panic!(r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`"#, $left, $right)
        }
    });
    ($left:expr, $right:expr,) => ({
        assert_eq_option!($left, $right)
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        if !cmp_eq_option!($left, $right) {
            panic!(r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{:?}`: {}"#, $left, $right, format_args!($($arg)+))
        }
    });
}
