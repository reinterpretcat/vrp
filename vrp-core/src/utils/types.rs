/// Represents a type with two values.
pub enum Either<L, R> {
    /// Left value.
    Left(L),
    /// Right value.
    Right(R),
}

impl<L, R> Clone for Either<L, R>
where
    L: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Either::Left(left) => Either::Left(left.clone()),
            Either::Right(right) => Either::Right(right.clone()),
        }
    }
}
