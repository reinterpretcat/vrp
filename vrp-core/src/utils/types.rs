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

impl<L, R, T> Iterator for Either<L, R>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = T>,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(it) => it.next(),
            Self::Right(it) => it.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Left(it) => it.size_hint(),
            Self::Right(it) => it.size_hint(),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Self::Left(it) => it.nth(n),
            Self::Right(it) => it.nth(n),
        }
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        match self {
            Self::Left(it) => it.fold(init, f),
            Self::Right(it) => it.fold(init, f),
        }
    }
}
