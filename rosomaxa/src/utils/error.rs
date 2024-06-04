use std::hash::{Hash, Hasher};

/// A basic error type which, essentially, a wrapper on String type.
#[derive(Clone, Debug)]
pub struct GenericError(String);

/// A type alias for result type with `GenericError`.
pub type GenericResult<T> = Result<T, GenericError>;

impl GenericError {
    /// Joins many errors with separator
    pub fn join_many(errs: &[GenericError], separator: &str) -> String {
        // TODO is there better way to have join method used?
        errs.iter().map(|err| err.0.clone()).collect::<Vec<_>>().join(separator)
    }
}

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GenericError {}

impl From<String> for GenericError {
    fn from(msg: String) -> Self {
        Self(msg)
    }
}

impl<'a> From<&'a str> for GenericError {
    fn from(value: &'a str) -> Self {
        Self(value.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for GenericError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self(value.to_string())
    }
}

impl From<std::io::Error> for GenericError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl PartialEq<Self> for GenericError {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for GenericError {}

impl Hash for GenericError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
