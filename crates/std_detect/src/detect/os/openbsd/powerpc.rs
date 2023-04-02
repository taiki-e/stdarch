//! Run-time feature detection for PowerPC on OpenBSD.
//!
//! https://github.com/openbsd/src/commit/0dc8bf2f295bba4fdf5278e4c00c8ffead354f47
//! https://github.com/openbsd/src/commit/e67f987282ba773958853d7b617ba42aaaac4ac0

// TODO: there is no powerpc-*-openbsd? https://doc.rust-lang.org/nightly/rustc/platform-support/openbsd.html

use crate::detect::cache;
use core::{mem::MaybeUninit, ptr};

// Defined in machine/cpu.h.
cfg_if::cfg_if! {
    if #[cfg(target_arch = "powerpc64")] {
        // https://github.com/openbsd/src/blob/72ccc03bd11da614f31f7ff76e3f6fce99bc1c79/sys/arch/powerpc64/include/cpu.h#L26-L30
        const CPU_ALTIVEC: libc::c_int = 1;
        const PPC_FEATURE2_ARCH_3_00: u32 = 0x00800000; // power9
    } else {
        // TODO: not in powerpc/include/cpu.h
        // https://github.com/openbsd/src/blob/72ccc03bd11da614f31f7ff76e3f6fce99bc1c79/sys/arch/macppc/include/cpu.h#L37-L44
        const CPU_ALTIVEC: libc::c_int = 2;
    }
}

/// Try to read the features from the system registers.
pub(crate) fn detect_features() -> cache::Initializer {
    let mut value = cache::Initializer::default();
    let enable_feature = |value: &mut cache::Initializer, f, enable| {
        if enable {
            value.set(f as u32);
        }
    };

    enable_feature(
        &mut value,
        Feature::altivec,
        sysctl_int(&[libc::CTL_MACHDEP, CPU_ALTIVEC]) == 1,
    );

    value
}

#[inline]
fn sysctl_int(mib: &[libc::c_int]) -> Option<libc::c_int> {
    const OUT_LEN: libc::size_t = core::mem::size_of::<libc::c_int>();
    let mut out = MaybeUninit::<libc::c_int>::uninit();
    let mut out_len = OUT_LEN;
    let res = unsafe {
        libc::sysctl(
            mib.as_ptr(),
            mib.len() as libc::c_uint,
            out.as_mut_ptr() as *mut libc::c_void,
            &mut out_len,
            ptr::null_mut(),
            0,
        )
    };
    if res == -1 || out_len != OUT_LEN {
        return None;
    }
    // SAFETY: we've checked that sysctl was successful and `out` was filled.
    Some(unsafe { out.assume_init() })
}
