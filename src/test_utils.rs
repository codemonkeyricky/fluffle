//! Test utilities for compile-time assertions.

/// Asserts that a type implements both `Send` and `Sync` traits.
/// This is a compile-time assertion that will fail to compile if the type
/// does not implement these traits, ensuring thread safety.
#[cfg(test)]
#[macro_export]
macro_rules! assert_send_sync {
    ($ty:ty) => {
        const _: fn() = || {
            fn assert_send_sync<T: Send + Sync>() {}
            assert_send_sync::<$ty>();
        };
    };
}