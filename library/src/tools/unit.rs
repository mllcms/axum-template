pub const KB: u64 = 1 << 10;
pub const MB: u64 = 1 << 20;
pub const GB: u64 = 1 << 30;
pub const TB: u64 = 1 << 40;
pub const UNIT: [&str; 4] = ["KB", "MB", "GB", "TB"];

pub fn unit(n: u64) -> String {
    let mut n = n as f64;
    for s in UNIT {
        n /= 1024.0;
        if 1024.0 > n {
            return format!("{n:.1}{s}");
        }
    }
    format!("{n:.1}{}", UNIT[3])
}

#[test]
fn unit_t() {
    assert_eq!("1.0KB", unit(KB));
    assert_eq!("1.0MB", unit(MB));
    assert_eq!("1.0GB", unit(GB));
    assert_eq!("1.0TB", unit(TB));
    assert_eq!("1025.0TB", unit(1025 * TB));
    assert_eq!("1.1MB", unit(MB + 100 * KB));
}
