//! Run-time feature detection on OpenBSD

// WIP: https://github.com/search?q=repo%3Aopenbsd%2Fsrc+%2FCTL_MACHDEP+def%2F&type=code

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        pub(crate) use self::aarch64::detect_features;
    } else if #[cfg(target_arch = "powerpc64")] {
        mod powerpc;
        pub(crate) use self::powerpc::detect_features;
    } else {
        use crate::detect::cache;
        /// Performs run-time feature detection.
        pub(crate) fn detect_features() -> cache::Initializer {
            cache::Initializer::default()
        }
    }
}
