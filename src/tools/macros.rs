#[macro_export]
macro_rules! gen_default {
    ($($id:ident, $v:expr $(;)?)*) => {
        $(
            fn $id() -> String {
                String::from($v)
            }
        )*
    };

    ($($id:ident, $v:expr, $t:ty $(;)?)*) => {
        $(
            fn $id() -> $t {
                $v
            }
        )*
    };
}
