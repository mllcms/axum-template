use std::sync::Arc;

pub trait CompareStr: Clone {
    fn compare(&self, uri: &str) -> bool;
}

impl<T: Fn(&str) -> bool + Clone> CompareStr for T {
    fn compare(&self, uri: &str) -> bool {
        self(uri)
    }
}

impl CompareStr for &str {
    fn compare(&self, uri: &str) -> bool {
        uri.contains(self)
    }
}

impl CompareStr for Arc<String> {
    fn compare(&self, uri: &str) -> bool {
        uri.contains(self.as_ref())
    }
}

impl<const N: usize> CompareStr for &'static [&str; N] {
    fn compare(&self, uri: &str) -> bool {
        self.iter().any(|a| uri.contains(a))
    }
}

impl CompareStr for Arc<Vec<String>> {
    fn compare(&self, uri: &str) -> bool {
        self.iter().any(|a| uri.contains(a))
    }
}
