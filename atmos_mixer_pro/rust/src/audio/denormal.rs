//! 오디오 처리 중 x86 비정규 실수(denormal)를 0으로 다루게 하는 가드(FTZ·DAZ).
//!
//! 소리가 멈추면 리버브·필터·리미터 상태가 지수적으로 줄어 비정규 실수 범위(약 1e-38 아래)에
//! 들어간다. x86 CPU는 비정규 실수 계산을 수십~수백 배 느리게 한다. 2026-10-09 Windows 측정
//! (40채널·1024프레임·채널 리버브 홀 80%, 1초 잡음 뒤 무음): 콜백 처리 시간이 평균 11ms에서
//! 12초 뒤 22ms, 44초 뒤 최대 98ms로 늘어 예산 21.3ms를 넘었고 90초까지 62ms에 머물렀다. 그동안
//! 장치는 이전 버퍼를 다시 내서 소리가 "버퍼가 안 맞는 것처럼" 늘어졌다. FTZ·DAZ를 켜면 내내 11ms다.
//! 비정규 실수는 −700dB 아래라 0으로 다뤄도 들리는 차이가 없다.
//!
//! Apple Silicon(aarch64)은 비정규 실수가 느려지지 않아 아무것도 하지 않는다(macOS 동작 그대로).

/// 만들 때 MXCSR에 FTZ(0x8000)·DAZ(0x0040)를 켜고, 버릴 때 원래 값으로 돌린다. 콜백은 장치
/// 드라이버의 스레드에서 돌기 때문에 끝나면 드라이버의 부동소수점 상태를 그대로 돌려준다.
/// 레지스터 읽기·쓰기만 한다(할당·잠금 없음, 오디오 스레드 3법칙).
pub struct DenormalGuard {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    saved_mxcsr: u32,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const FTZ_DAZ: u32 = 0x8040;

impl DenormalGuard {
    #[inline]
    pub fn new() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let mut saved_mxcsr: u32 = 0;
            // SAFETY: stmxcsr는 MXCSR 4바이트를 지역 변수에 쓰고, ldmxcsr는 지역 변수에서 읽는다.
            // 둘 다 SSE가 있는 모든 x86_64 CPU에서 유효하고 다른 메모리를 건드리지 않는다.
            unsafe {
                core::arch::asm!(
                    "stmxcsr [{}]",
                    in(reg) &mut saved_mxcsr,
                    options(nostack, preserves_flags)
                );
                let flushed = saved_mxcsr | FTZ_DAZ;
                core::arch::asm!(
                    "ldmxcsr [{}]",
                    in(reg) &flushed,
                    options(nostack, readonly, preserves_flags)
                );
            }
            Self { saved_mxcsr }
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            Self {}
        }
    }
}

impl Default for DenormalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DenormalGuard {
    #[inline]
    fn drop(&mut self) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        // SAFETY: new()에서 읽어 둔 원래 MXCSR 값을 되돌린다.
        unsafe {
            core::arch::asm!(
                "ldmxcsr [{}]",
                in(reg) &self.saved_mxcsr,
                options(nostack, readonly, preserves_flags)
            );
        }
    }
}
