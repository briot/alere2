/// Whether the vector contains all-equal elements
pub fn is_all_same<T: PartialEq>(arr: &[T]) -> bool {
    match arr {
        [] => true,
        [first, ..] => arr.iter().all(|v| v == first),
    }
}
