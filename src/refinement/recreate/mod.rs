use crate::construction::states::InsertionContext;

pub trait Recreate {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod recreate_with_cheapest;
pub use self::recreate_with_cheapest::RecreateWithCheapest;
