use std::cmp::Ordering;

pub(crate) fn cmp_ignore_case(left: &str, right: &str) -> Ordering {
    fn to_ascii_lowercase(s: &str) -> impl Iterator<Item = u8> {
        s.bytes().map(|byte| byte.to_ascii_lowercase())
    }

    if left.is_ascii() && right.is_ascii() {
        to_ascii_lowercase(left).cmp(to_ascii_lowercase(right))
    } else {
        left.to_lowercase().cmp(&right.to_lowercase())
    }
}
