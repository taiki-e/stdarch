//! Run-time feature detection for Aarch64 on Fuchsia.
//!
//! Refs:
//! - https://fuchsia.dev/fuchsia-src/reference/syscalls/system_get_features
//! - https://reviews.llvm.org/D118839

use crate::detect::{cache, Feature};

// https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/zircon/system/public/zircon/types.h
#[allow(non_camel_case_types)]
type zx_status_t = i32;

// https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/zircon/system/public/zircon/errors.h
const ZX_OK: zx_status_t = 0;
// https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/zircon/system/public/zircon/features.h
const ZX_FEATURE_KIND_CPU: u32 = 0;
const ZX_ARM64_FEATURE_ISA_FP: u32 = 1 << 1;
const ZX_ARM64_FEATURE_ISA_ASIMD: u32 = 1 << 2;
const ZX_ARM64_FEATURE_ISA_AES: u32 = 1 << 3;
const ZX_ARM64_FEATURE_ISA_PMULL: u32 = 1 << 4;
const ZX_ARM64_FEATURE_ISA_SHA1: u32 = 1 << 5;
const ZX_ARM64_FEATURE_ISA_SHA256: u32 = 1 << 6;
const ZX_ARM64_FEATURE_ISA_CRC32: u32 = 1 << 7;
const ZX_ARM64_FEATURE_ISA_ATOMICS: u32 = 1 << 8;
const ZX_ARM64_FEATURE_ISA_RDM: u32 = 1 << 9;
const ZX_ARM64_FEATURE_ISA_SHA3: u32 = 1 << 10;
const ZX_ARM64_FEATURE_ISA_SM3: u32 = 1 << 11;
const ZX_ARM64_FEATURE_ISA_SM4: u32 = 1 << 12;
const ZX_ARM64_FEATURE_ISA_DP: u32 = 1 << 13;
const ZX_ARM64_FEATURE_ISA_DPB: u32 = 1 << 14;
const ZX_ARM64_FEATURE_ISA_FHM: u32 = 1 << 15;
const ZX_ARM64_FEATURE_ISA_TS: u32 = 1 << 16;
const ZX_ARM64_FEATURE_ISA_RNDR: u32 = 1 << 17;
const ZX_ARM64_FEATURE_ISA_SHA512: u32 = 1 << 18;

#[link(name = "zircon")]
extern "C" {
    // https://fuchsia.dev/fuchsia-src/reference/syscalls/system_get_features
    fn zx_system_get_features(kind: u32, features: *mut u32) -> zx_status_t;
}

/// Try to read the features using zx_system_get_features.
pub(crate) fn detect_features() -> cache::Initializer {
    let mut value = cache::Initializer::default();

    let mut features = 0_u32;
    let res = unsafe { zx_system_get_features(ZX_FEATURE_KIND_CPU, &mut features) };
    if res != ZX_OK {
        return value;
    }

    let mut enable_feature = |f, enable| {
        if enable {
            value.set(f as u32);
        }
    };

    enable_feature(Feature::fp, features & ZX_ARM64_FEATURE_ISA_FP != 0);
    enable_feature(Feature::asimd, features & ZX_ARM64_FEATURE_ISA_ASIMD != 0);
    enable_feature(Feature::pmull, features & ZX_ARM64_FEATURE_ISA_PMULL != 0);
    enable_feature(Feature::crc, features & ZX_ARM64_FEATURE_ISA_CRC32 != 0);
    enable_feature(Feature::lse, features & ZX_ARM64_FEATURE_ISA_ATOMICS != 0);
    enable_feature(Feature::rdm, features & ZX_ARM64_FEATURE_ISA_RDM != 0);
    enable_feature(Feature::dotprod, features & ZX_ARM64_FEATURE_ISA_DP != 0);
    enable_feature(Feature::dpb, features & ZX_ARM64_FEATURE_ISA_DPB != 0);
    enable_feature(Feature::fhm, features & ZX_ARM64_FEATURE_ISA_FHM != 0);
    enable_feature(Feature::flagm, features & ZX_ARM64_FEATURE_ISA_TS != 0);
    enable_feature(Feature::rand, features & ZX_ARM64_FEATURE_ISA_RNDR != 0);

    enable_feature(Feature::aes, features & ZX_ARM64_FEATURE_ISA_AES != 0);
    let sha1 = features & ZX_ARM64_FEATURE_ISA_SHA1 != 0;
    let sha2 = features & ZX_ARM64_FEATURE_ISA_SHA256 != 0;
    enable_feature(Feature::sha2, sha1 && sha2);
    let sha512 = features & ZX_ARM64_FEATURE_ISA_SHA512 != 0;
    let sha3 = features & ZX_ARM64_FEATURE_ISA_SHA3 != 0;
    enable_feature(Feature::sha3, sha1 && sha2 && sha512 && sha3);
    let sm3 = features & ZX_ARM64_FEATURE_ISA_SM3 != 0;
    let sm4 = features & ZX_ARM64_FEATURE_ISA_SM4 != 0;
    enable_feature(Feature::sm4, sm3 && sm4);

    value
}
