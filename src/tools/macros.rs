/// 生成默认值函数
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

/// 再次导出
#[macro_export]
macro_rules! re_export {
    ($($vis:vis mod $module:ident;)*) => {
        $(
            $vis mod $module;
            pub use $module::*;
        )*
    };
}
