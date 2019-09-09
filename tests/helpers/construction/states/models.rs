use crate::construction::states::InsertionProgress;

pub fn test_insertion_progress() -> InsertionProgress {
    InsertionProgress {
        cost: 1000.0,
        completeness: 1.0,
        total: 1,
    }
}
