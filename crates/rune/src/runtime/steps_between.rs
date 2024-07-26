pub(crate) trait StepsBetween {
    fn steps_between(start: Self, end: Self) -> Option<usize>;
}

impl StepsBetween for i64 {
    #[inline]
    fn steps_between(start: Self, end: Self) -> Option<usize> {
        usize::try_from(end.checked_sub(start)?).ok()
    }
}

impl StepsBetween for u8 {
    #[inline]
    fn steps_between(start: Self, end: Self) -> Option<usize> {
        Some(usize::from(end.checked_sub(start)?))
    }
}
