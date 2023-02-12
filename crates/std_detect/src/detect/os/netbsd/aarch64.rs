//! Run-time feature detection for Aarch64 on NetBSD.
//!
//! NetBSD doesn't trap the mrs instruction, but exposes the system registers through sysctl.
//! https://github.com/NetBSD/src/commit/70057642485887b9601804adb983c70d11e48d9a
//! https://github.com/golang/sys/commit/ef9fd89ba245e184bdd308f7f2b4f3c551fa5b0f

use crate::detect::cache;
use core::{mem::MaybeUninit, ptr};

// Defined in aarch64/armreg.h.
// https://github.com/NetBSD/src/blob/49ff686c908df01d34af98d7a46d51aabe7008fa/sys/arch/aarch64/include/armreg.h#L1626
#[derive(Clone, Copy)]
#[repr(C)]
struct aarch64_sysctl_cpu_id {
    // NetBSD 9.0+
    // https://github.com/NetBSD/src/commit/70057642485887b9601804adb983c70d11e48d9a
    _midr: u64,      /* Main ID Register */
    _revidr: u64,    /* Revision ID Register */
    _mpidr: u64,     /* Multiprocessor Affinity Register */
    _aa64dfr0: u64,  /* A64 Debug Feature Register 0 */
    _aa64dfr1: u64,  /* A64 Debug Feature Register 1 */
    aa64isar0: u64,  /* A64 Instruction Set Attribute Register 0 */
    aa64isar1: u64,  /* A64 Instruction Set Attribute Register 1 */
    _aa64mmfr0: u64, /* A64 Memory Model Feature Register 0 */
    _aa64mmfr1: u64, /* A64 Memory Model Feature Register 1 */
    _aa64mmfr2: u64, /* A64 Memory Model Feature Register 2 */
    _aa64pfr0: u64,  /* A64 Processor Feature Register 0 */
    _aa64pfr1: u64,  /* A64 Processor Feature Register 1 */
    _aa64zfr0: u64,  /* A64 SVE Feature ID Register 0 */
    _mvfr0: u32,     /* Media and VFP Feature Register 0 */
    _mvfr1: u32,     /* Media and VFP Feature Register 1 */
    _mvfr2: u32,     /* Media and VFP Feature Register 2 */
    // NetBSD 10.0+
    // https://github.com/NetBSD/src/commit/908d3fadf77e1b392db35d1076f7f9961c1d351d
    _pad: u32,
    _clidr: u64, /* Cache Level ID Register */
    _ctr: u64,   /* Cache Type Register */
}

/// Try to read the features from the system registers.
pub(crate) fn detect_features() -> cache::Initializer {
    let mut cpu_id_buf: aarch64_sysctl_cpu_id = unsafe { core::mem::zeroed() };
    // First, get system registers for cpu0.
    // If failed, returns default because machdep.cpuN.cpu_id sysctl is not available.
    // machdep.cpuN.cpu_id sysctl was added on NetBSD 9.0 so it is not available on older versions.
    let cpu0 = match unsafe { sysctl_cpu_id(&mut cpu_id_buf, b"machdep.cpu0.cpu_id\0") } {
        Some(cpu0) => cpu0,
        None => return cache::Initializer::default(),
    };
    // Second, get the number of cpus.
    // If failed, returns default because nothing can be assumed about the other cores.
    // Do not use available_parallelism/_SC_NPROCESSORS_ONLN/HW_NCPUONLINE because
    // offline cores may become online during execution.
    let cpus = match sysctl32(&[libc::CTL_HW, libc::HW_NCPU]) {
        Some(0) | None => return cache::Initializer::default(), // failed
        Some(1) => {
            // single-core
            return super::aarch64::parse_system_registers(cpu0.0, cpu0.1, cpu0.2, Some(cpu0.3));
        }
        Some(cpus) => cpus,
    };
    // Unfortunately, there is a bug in Samsung's SoC that supports
    // different CPU features in big and little cores.
    // https://web.archive.org/web/20210908112244/https://medium.com/@niaow/a-big-little-problem-a-tale-of-big-little-gone-wrong-e7778ce744bb
    // https://github.com/golang/go/issues/28431#issuecomment-433573689
    // https://en.wikichip.org/wiki/samsung/exynos/9810
    // So, make sure that all cores provide the same CPU features.
    // Note that we are only checking the consistency of the registers to
    // which we actually refer. (If we check all registers, fields such as
    // product variant are also checked, which breaks runtime detection on
    // most big.LITTLE SoCs.)
    let mut name_buf = MachdepNameBuffer::new();
    for n in 1..cpus {
        let cpu = match unsafe { sysctl_cpu_id(&mut cpu_id_buf, name_buf.name(n)) } {
            Some(cpu) => cpu,
            None => return cache::Initializer::default(),
        };
        if cpu != cpu0 {
            return cache::Initializer::default();
        }
    }

    super::aarch64::parse_system_registers(cpu0.0, cpu0.1, cpu0.2, Some(cpu0.3))
}

#[inline]
fn sysctl32(mib: &[libc::c_int]) -> Option<u32> {
    const OUT_LEN: libc::size_t = core::mem::size_of::<u32>() as libc::size_t;
    let mut out = MaybeUninit::<u32>::uninit();
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

#[inline]
unsafe fn sysctl_cpu_id(buf: &mut aarch64_sysctl_cpu_id, name: &[u8]) -> Option<AA64Reg> {
    const OUT_LEN: libc::c_size_t = core::mem::size_of::<aarch64_sysctl_cpu_id>() as libc::c_size_t;

    debug_assert_eq!(name.last(), Some(&0), "{:?}", name);
    debug_assert_eq!(name.iter().filter(|&&v| v == 0).count(), 1, "{:?}", name);

    let mut out_len = OUT_LEN;
    let res = unsafe {
        libc::sysctlbyname(
            name.as_ptr() as *const libc::c_char,
            buf as *mut _ as *mut libc::c_void,
            &mut out_len,
            ptr::null_mut(),
            0,
        )
    };
    if res != 0 {
        return None;
    }
    Some((buf.aa64isar0, buf.aa64isar1, buf.aa64mmfr2, buf.aa64pfr0))
}

struct MachdepNameBuffer {
    // MaybeUninit could be used here, but this is already efficient enough
    // since it reuses buffers, and there is not much benefit compared to
    // the amount of unsafe code it increases.
    // https://godbolt.org/z/d3s349vET
    buf: [u8; NAME_MAX_LEN],
}

const NAME_PREFIX: &[u8] = b"machdep.cpu";
const NAME_SUFFIX: &[u8] = b".cpu_id\0";
const U32_MAX_LEN: usize = 10;
const NAME_MAX_LEN: usize = NAME_PREFIX.len() + NAME_SUFFIX.len() + U32_MAX_LEN;
impl MachdepNameBuffer {
    #[inline]
    fn new() -> Self {
        let mut buf: [u8; NAME_MAX_LEN] = [0; NAME_MAX_LEN];
        buf[..NAME_PREFIX.len()].copy_from_slice(NAME_PREFIX);
        Self { buf }
    }

    #[inline]
    fn name(&mut self, mut cpu: u32) -> &[u8] {
        let mut len = NAME_PREFIX.len();
        macro_rules! put {
            ($cur:tt $($tt:tt)*) => {
                if cpu >= $cur {
                    put!($($tt)*);
                    let n = cpu / $cur;
                    self.buf[len] = (n as u8) + b'0';
                    len += 1;
                    cpu %= $cur;
                }
            };
            () => {};
        }
        put!(
            10
            100
            1000
            10000
            100000
            1000000
            10000000
            100000000
            1000000000
        );
        self.buf[len] = (cpu as u8) + b'0';
        len += 1;
        self.buf[len..len + NAME_SUFFIX.len()].copy_from_slice(NAME_SUFFIX);
        len += NAME_SUFFIX.len();
        &self.buf[..len]
    }
}
