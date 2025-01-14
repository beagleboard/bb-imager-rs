cfg_if::cfg_if! {
    if #[cfg(feature = "pb2_mspm0_raw")] {
        mod raw;
        pub use raw::*;
    } else if #[cfg(feature = "pb2_mspm0_dbus")] {
        mod dbus;
        pub use dbus::*;
    }
}
