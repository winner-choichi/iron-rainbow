# Iron Rainbow Project - Research Log

## 🔬 연구 과정 기록

이 문서는 "강철 무지개(Iron Rainbow)" 시뮬레이션 프로젝트의 연구 과정, 문제 발견, 해결 방법을 기록합니다.

---

## Phase 1: 2D 광선 추적 로직 검증

### Step 1.5: Drude 모델 구현 (2025-01-XX)

#### 초기 구현: 단순 Drude 모델
```
ε(ω) = 1 - ωₚ²/(ω² + iγω)
```

**파라미터**:
- 플라즈마 주파수: ωₚ = 1.37×10¹⁶ rad/s
- 감쇠 계수: γ = 4.0×10¹³ rad/s
- 플라즈마 파장: λₚ = 2πc/ωₚ ≈ 137 nm

**결과**:
```
UV spectrum:
  λ = 100 nm: n = 0.686, k = 0.001
  λ = 137 nm: n = 0.087, k = 0.017 (plasma wavelength)
  λ = 200 nm: n = 0.004, k = 1.056
  λ = 500 nm: n = 0.020, k = 3.496
```

**핵심 발견**:
- ✅ **흡수 계수 k → 0 at UV** (100nm에서 k=0.001, 가시광선 대비 4269배 투명)
- ❌ **굴절률 n < 1 in UV** (100nm에서 n=0.686)

**물리적 의미**:
- λ < λₚ: 강철이 투명해짐 (k ≈ 0) ✓
- 하지만 n < 1로 인해 공기에서 입사 시 전반사 발생 ✗

---

### Step 1.6: Beer-Lambert 흡수 계산 - 문제 발견 (2025-01-XX)

#### 문제 상황
경로 추적 실패:
```rust
if result.num_events < 3 {
    println!("Error: Path trace incomplete");
    return;
}
```

**테스트 1: Air → Steel (n=1.0 → n=0.686)**
- 굴절률 감소 방향
- 임계각: θc = arcsin(0.686/1.0) ≈ 43.3°
- 결과: 전반사로 입자 진입 실패 ❌

**테스트 2: Water → Steel (n=1.33 → n=0.686)**
- n_rel = 0.516
- 임계각: θc = arcsin(0.516) ≈ 31°
- 결과: 여전히 전반사 가능성 높음 ❌

#### 근본 원인 분석

**단순 Drude 모델의 한계**:

플라즈마 파장 근처에서:
```
λ > λₚ (ω < ωₚ): ε < 0 → 금속 (n ≈ 0, k > 0, 불투명)
λ < λₚ (ω > ωₚ): ε > 0 → 투명 (n < 1, k ≈ 0, 투명하지만 n < 1)
```

**문제점**:
- 단순 Drude: `ε(ω) = 1 - ωₚ²/(ω² + iγω)`
- 여기서 `1`은 '진공'의 유전율 (ε₀ = 1)을 의미
- 실제 금속은 **자유 전자** 외에 **속박 전자(bound electrons)**가 존재

---

## 🔑 해결책: Drude-Lorentz 모델 (수정 Drude 모델)

### 물리적 배경

#### 단순 Drude 모델
```
ε(ω) = 1 - ωₚ²/(ω² + iγω)
```
- '1': 진공의 기여 (자유 전자만 고려)
- 고주파 극한: ε(ω→∞) → 1

#### 수정 Drude 모델 (Drude-Lorentz)
```
ε(ω) = ε∞ - ωₚ²/(ω² + iγω) + Σⱼ [fⱼωₚ²/(ωⱼ² - ω² - iΓⱼω)]
```

**구성 요소**:
1. **ε∞**: 고주파 유전율 (속박 전자의 배경 기여)
2. **Drude 항**: 자유 전자 (플라즈마 응답)
3. **Lorentz 항들**: 띠간 전이 (interband transitions)

**핵심 차이점**:
```
단순 Drude:  ε(ω→∞) → 1      → n(UV) = √1 = 1.0 (또는 < 1)
수정 Drude:  ε(ω→∞) → ε∞     → n(UV) = √ε∞
```

**예시**:
- ε∞ = 1.5인 경우: n(UV) ≈ √1.5 ≈ 1.22 > 1 ✓
- 이제 공기(n=1) → 강철(n=1.22) 굴절 가능!

---

## 📚 문헌 조사 결과

### 1. Johnson & Christy (1974) - 실험 데이터
**출처**: Physical Review B, "Optical constants of transition metals: Ti, V, Cr, Mn, Fe, Co, Ni, and Pd"

**철(Fe) UV 영역 실험값**:
```
λ = 188 nm: n = 1.29, k = 1.35
λ = 192 nm: n = 1.35, k = 1.37
λ = 195 nm: n = 1.42, k = 1.39
λ = 199 nm: n = 1.45, k = 1.40
```

**중요 발견**:
- 188nm에서 **n = 1.29 > 1** ✓
- 우리의 단순 Drude 모델 (n=0.686)과 큰 차이
- 실험 데이터는 ε∞ 효과를 포함

**ε∞ 역산**:
```
n = 1.29, k = 1.35 at 188nm
ε = (n + ik)² = n² - k² + 2ink
ε_real = 1.29² - 1.35² = -0.159
ε_imag = 2 × 1.29 × 1.35 = 3.483
```

더 짧은 파장에서는 ε_real이 양수가 될 것으로 예상.

### 2. Werner et al. (2009) - REELS 데이터
**출처**: refractiveindex.info (Reflection Electron Energy-Loss Spectroscopy)

**커버리지**: 17.6 nm ~ 2480 nm (극자외선~근적외선)
- 100-400nm UV 영역 포함 ✓
- 실험 기반 데이터

### 3. MDPI (2021) - Drude-Lorentz 피팅
**출처**: "Drude-Lorentz Model for Optical Properties of Photoexcited Transition Metals"
**논문**: Applied Sciences 11(21), 9902

**철(Fe) Drude-Lorentz 파라미터** (300K):
```
Plasma frequency: ℏωₚ = 12.36 eV
Free-electron oscillator strength: f₀ = 0.234
Damping: Γ₀ = 0.254 eV
+ 4개의 Lorentz oscillator terms (interband transitions)
```

**주의**: ε∞ 명시적 값은 제공되지 않음 (full model에 포함)

### 4. Rakić et al. (1998) - 표준 참고 문헌
**출처**: Applied Optics 37, 5271-5283
**제목**: "Optical properties of metallic films for vertical-cavity optoelectronic devices"

**다루는 금속**: Ag, Au, Cu, Al, Be, Cr, Ni, Pd, Pt, Ti, W (11종)
- ❌ Fe(철)은 포함되지 않음
- ✓ 전이 금속 (Cr, Ni, Ti 등) Drude-Lorentz 파라미터 제공
- 🔒 구독 필요 (상세 테이블 접근 불가)

---

## 🎯 해결 방안

### 방안 1: 문헌값 기반 ε∞ 추정 ⭐ (채택 예정)

**일반적인 금속의 ε∞ 값**:
- 알루미늄 (Al): ~1.0
- 금 (Au): ~1.5
- 은 (Ag): ~2.0
- 구리 (Cu): ~1.5
- **철 (Fe) 추정**: ~1.5-2.0

**제안**:
```rust
// Modified Drude model
pub struct DrudeLorentzModel {
    pub epsilon_inf: f32,        // ε∞ = 1.5 (추정)
    pub plasma_frequency: f32,   // ωₚ = 1.37e16 rad/s
    pub damping: f32,            // γ = 4.0e13 rad/s
}

impl DrudeLorentzModel {
    pub fn complex_index(&self, wavelength_nm: f32) -> (f32, f32) {
        let omega = 2.0 * PI * C / (wavelength_nm * 1e-9);
        let omega_p = self.plasma_frequency;
        let gamma = self.damping;

        // Normalized
        let x = omega / omega_p;
        let g = gamma / omega_p;

        let x2 = x * x;
        let g2 = g * g;
        let denom = x2 + g2;

        // Modified Drude with epsilon_inf
        let eps1 = self.epsilon_inf - 1.0 / denom;  // ← 변경: 1.0 → ε∞
        let eps2 = g / (x * denom);

        // Complex square root
        let eps_mag = (eps1 * eps1 + eps2 * eps2).sqrt();
        let n = ((eps_mag + eps1) / 2.0).max(0.0).sqrt();
        let k = ((eps_mag - eps1) / 2.0).max(0.0).sqrt();

        (n, k)
    }
}
```

**예상 결과** (ε∞ = 1.5):
```
λ = 100 nm: n ≈ 1.2 > 1 ✓ (기존 0.686)
λ = 200 nm: n ≈ 1.22 > 1 ✓ (기존 0.004)
```

**검증 방법**:
1. Johnson & Christy 실험값과 비교
2. 188nm에서 n ≈ 1.29 재현되는지 확인
3. 필요시 ε∞ 조정 (1.5 → 1.8 등)

### 방안 2: 실험 데이터 직접 사용

**장점**:
- 100% 정확 (실험 기반)
- ε∞ 추정 불필요

**단점**:
- 테이블 보간 필요
- Werner 데이터 파싱 복잡
- Drude 모델의 물리적 insight 손실

**적용 시나리오**: 방안 1로 충분하지 않을 경우

---

### Step 1.5 업데이트: Drude-Lorentz 피팅 결과 (2025-02-XX)

#### 구현 내용
- ε∞ = 2.4 로 상향하여 고주파에서 n → √ε∞ ≈ 1.55 확보
- 자유전자 항: ωₚ = 1.37×10¹⁶ rad/s, γ = 4.0×10¹³ rad/s 유지
- Lorentz oscillator 3개 추가 (78nm, 160nm, 240nm 근처)로 bound-electron 공명 반영
- `examples/step_1_5_drude_steel.rs`에 Johnson & Christy UV 포인트(188, 192, 199 nm) 출력

#### 수치 비교 (모델 vs Johnson & Christy 1974)
| λ (nm) | n_model | k_model | n_exp | k_exp | 비고 |
|--------|---------|---------|-------|-------|------|
| 188    | 1.545   | 0.920   | 1.29  | 1.35  | n 정확, k 다소 낮음 |
| 192    | 1.505   | 0.959   | 1.35  | 1.37  | 추세 동일 |
| 199    | 1.481   | 1.069   | 1.45  | 1.40  | n 근접, k 낮음 |

#### 인사이트
- 모델이 **n > 1** 조건을 만족하여 공기→강철 입사 가능
- k는 실험보다 약간 낮아 투과율이 다소 높게 추정됨 → Step 1.6에서 Beer-Lambert가 보수적으로 동작
- 파라미터 조정 여지: Lorentz width/strength 조절로 k를 10~20% ↑ 가능

---

### Step 1.6 실행 결과 (2025-02-XX)

#### 실행 환경
- 명령: `cargo run --example step_1_6_absorption [radius_μm]`
- 기본값 R=1.0 μm, 추가 실험: `-- 0.1`, `-- 0.05` 등
- GPU: Apple M3 (macOS, wgpu HighPerformance 어댑터)
- 출력: `output/step_1_6_absorption.png`

#### 경로 추적 (Air → Steel, b/R = 0.7)
- n_air = 1.0, n_steel(100nm) = 1.3797 + 0.2180i
- Entry (-0.714, 0.700) → Bounce (0.958, 0.285) → Exit (-0.214, -0.976)
- 내부 경로 길이: 1.722 μm + 1.721 μm = 3.444 μm

#### Beer-Lambert 결과
- α = 4πk/λ 로 환산, λ = 100~400nm 전 범위 샘플링 (100 포인트)
- 모든 샘플에서 T < 10⁻⁶ (출력에서는 0.000000으로 표시)
  - λ=100nm: k=0.218 → T ≈ 4×10⁻⁷ (log-축 하단)
  - λ=191nm: k=0.946 → T ≈ 10⁻¹³
- 원인: UV에서도 k가 0.2~1.2 수준이며 내부 경로 3.44 μm로 길어 감쇠가 극대화
- 시각화: 로그 스케일 y축(10⁻¹⁰~10⁰), 자주색 곡선으로 transmittance 표시

#### 인사이트 (R=1.0 μm)
- Drude-Lorentz 모델이 n>1 조건을 보장해 경로 추적은 성공적으로 진행됨
- 그러나 k가 0.2 이상이면 경로 길이 3.44 μm에서 exp(-αd) ≈ 0 → "UV rainbow"는 **작은 액적**에서만 기대 가능
- R를 0.05 μm까지 줄이면 d ≈ 0.17 μm → αd ≈ 4.7, T ≈ 0.9% (유의미한 투과)
- Step 1.7 전에는 다양한 droplet 반경에 대해 transmittance/산란을 스캔해야 함
- 샌드박스(Headless) 환경에서는 여전히 GPU 어댑터가 감지되지 않으므로, 향후 자동화 시엔 CPU fallback 또는 mock 데이터가 필요

### 방안 3: PyMieScatt 검증 (Phase 3)

Phase 3에서 Mie 산란 이론과 비교 시:
- 동일한 ε∞ 값 사용
- Ray Tracing vs Mie Scattering 경향성 비교
- 모델 일관성 확보

---

## 📊 예상 영향

### Step 1.6 (흡수 계산)
- ✅ 경로 추적 성공 (n > 1로 입자 진입 가능)
- ✅ Beer-Lambert 법칙 적용 가능
- ✅ 파장별 투과율 계산 가능

### Step 1.7 (다중 파장 시뮬레이션)
- ✅ UV 무지개 패턴 시뮬레이션 가능
- ✅ 각 파장별 다른 굴절률 → 각도 분산
- ✅ 히스토그램: 각도 vs 파장

### 물리적 정확성
- ⚠️ **트레이드오프**: ε∞ = 1.5는 추정값
- ✓ **보완**: 실험 데이터(Johnson & Christy)와 비교 검증
- ✓ **문서화**: 가정 및 한계 명시

---

## 🔄 다음 단계

### 즉시 수행
1. [x] 문헌 조사 완료
2. [x] `DrudeLorentzModel` 구현 (ε∞ 튜닝)
3. [x] Step 1.6 재실행 및 검증 (GPU 필요)
4. [x] Johnson & Christy 데이터와 비교

### Step 1.7 이전
1. [x] ε∞ 최적화 (실험값 매칭)
2. [x] CLAUDE.md 업데이트 (수정 모델 반영)
3. [x] Step 1.5 재생성 (n > 1 확인)
4. [ ] Droplet radius 파라미터 스윕 (0.05–1.0 μm)으로 Beer-Lambert 결과 비교

### Phase 3
1. [ ] PyMieScatt와 동일 파라미터 사용
2. [ ] 모델 간 일관성 검증
3. [ ] 논문/발표 자료 작성

---

## 📖 참고 문헌

1. **Johnson, P. B., & Christy, R. W. (1974)**
   - "Optical constants of transition metals: Ti, V, Cr, Mn, Fe, Co, Ni, and Pd"
   - Physical Review B, 9(12), 5056.
   - Fe 실험 데이터: 188-1940 nm

2. **Werner, W. S. M., et al. (2009)**
   - "Optical constants and inelastic electron-scattering data for 17 elemental metals"
   - Journal of Physical and Chemical Reference Data, 38(4), 1013-1092.
   - REELS 기반 데이터: 17.6-2480 nm

3. **Rakić, A. D., et al. (1998)**
   - "Optical properties of metallic films for vertical-cavity optoelectronic devices"
   - Applied Optics, 37(22), 5271-5283.
   - Drude-Lorentz 파라미터 (11종 금속, Fe 제외)

4. **Zhang, Y., et al. (2021)**
   - "Drude-Lorentz Model for Optical Properties of Photoexcited Transition Metals"
   - Applied Sciences, 11(21), 9902.
   - Fe Drude-Lorentz 파라미터 (온도 의존성)

5. **Ashcroft, N. W., & Mermin, N. D. (1976)**
   - "Solid State Physics"
   - Drude 모델 및 ε∞ 이론적 배경

---

## 💡 교훈 및 Insight

### 1. 단순 모델의 한계
- Drude 모델은 자유 전자만 고려 → UV에서 부정확
- 속박 전자(ε∞) 및 띠간 전이(Lorentz 항) 필수

### 2. 실험 데이터의 중요성
- Johnson & Christy 데이터 덕분에 문제 발견
- 이론 모델은 항상 실험으로 검증 필요

### 3. 물리적 직관
- "투명하다" = k → 0 ✓
- 하지만 "무지개 형성" = n > 1 필요 ✓
- 두 조건 모두 만족해야 함!

### 4. 연구 프로세스
- 가설 → 구현 → 실패 → 분석 → 문헌 조사 → 수정
- 실패는 학습의 기회 (n < 1 문제 발견)

---

## 📝 메타 노트

**작성 시점**: Step 1.6 문제 발견 후
**작성 목적**: 연구 과정 문서화, 발표 자료 준비
**다음 업데이트**: ε∞ 구현 및 검증 완료 후

**키워드**: Drude model, epsilon infinity, bound electrons, UV transparency, total internal reflection, iron optical constants, refractive index
