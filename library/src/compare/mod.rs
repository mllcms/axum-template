crate::re_export! {
    mod str;
}
pub fn always_true(_: &str) -> bool {
    true
}

pub fn always_false(_: &str) -> bool {
    false
}
