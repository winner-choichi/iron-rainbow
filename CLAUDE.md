# Iron Rainbow Simulator - Development Plan

## 🎯 최종 목표: "강철 무지개(Iron Rainbow)" 시뮬레이션

**액체 강철(Liquid Steel) 입자에 의해 생성되는 무지개**를 물리적으로 정확하게 시뮬레이션하고 렌더링

### 핵심 물리학

1. **광선 추적 (Ray Tracing)**
   - 빛이 구형 액체 강철 입자에 입사 → 굴절 → 내부 반사 → 재굴절
   - 스넬의 법칙(Snell's Law) 적용

2. **분산 (Dispersion)**
   - 강철의 굴절률 n(λ)은 파장에 따라 변함
   - Drude 모델 기반 계산

3. **흡수 (Absorption)**
   - 강철은 불투명한 금속 → 빛이 내부를 통과하며 강하게 감쇠
   - 흡수 계수 k(λ) 역시 파장 의존적
   - 비어-램버트 법칙(Beer-Lambert Law)

### 핵심 가설

> **"가시광선에서는 흡수(k)가 너무 강해 투과 불가능하지만,
> 원적외선(IR) 영역에서는 k가 낮아져 투과 가능하고,
> n의 분산에 의해 무지개 형성 가능"**

---

## 🗺️ 3단계 개발 계획

### 기술 스택 (통일)
- **언어**: Rust
- **GPU 컴퓨팅**: wgpu (WGSL 컴퓨트 셰이더)
- **이미지 출력**: image 크레이트
- **비동기 실행**: pollster
- **Git 브랜치**: Phase별 독립 브랜치

---

## Phase 1: 2D CPU 로직 검증 (branch: `phase1-2d-logic`)

### 목표
- 3D 렌더링 없이 2D 평면에서 광선 추적 로직 구현
- wgpu 컴퓨트 셰이더로 GPU 병렬 처리
- 결과를 1D 히스토그램 또는 2D 스펙트로그램(각도 vs 파장) 이미지로 저장
- 물리 로직 검증

### Micro-steps

#### ✅ Step 1.0: 프로젝트 초기화
- [x] Git 저장소 초기화
- [x] `phase1-2d-logic` 브랜치 생성
- [x] Cargo 프로젝트 생성
- [x] 의존성 추가: wgpu, pollster, image, bytemuck, futures-intrusive
- [x] 검증: `cargo run` 실행 확인

#### ✅ Step 1.1: wgpu 초기화 및 "Hello GPU"
- [x] wgpu 인스턴스/어댑터/디바이스/큐 설정
- [x] 간단한 컴퓨트 셰이더 작성 (각 스레드가 ID × 2.0 계산)
- [x] GPU 버퍼 생성 및 바인드 그룹 설정
- [x] 컴퓨트 셰이더 실행 및 CPU로 결과 읽기
- [x] 검증: 콘솔에 GPU 계산 결과 출력 (256개 값)
- [x] GPU: Apple M3 확인
- [x] 결과: output[i] = i * 2.0 검증 성공

#### ✅ Step 1.2: 2D 광선-원 교차 계산 (WGSL)
- [x] 광선 구조체 정의 (origin, direction)
- [x] 원(강철 입자) 구조체 정의 (center, radius)
- [x] 광선-원 교차 알고리즘 구현 (해석적 해법, 이차방정식 판별식)
- [x] 검증: 16개 테스트 케이스 (13 hit, 3 miss)
- [x] 코드 구조화: 전문적인 모듈 시스템 구축
  - `src/gpu/`: GPU 컨텍스트 및 파이프라인 관리
  - `src/shaders/`: WGSL 셰이더 파일
  - `src/geometry/`: Ray, Circle 기하학 구조체
  - `src/physics/`: 물리 알고리즘 (교차, 굴절)
  - `src/visualization/`: 2D 렌더링 엔진
- [x] 시각화: `examples/step_1_2_intersection.rs`
  - 출력: `output/step_1_2_intersection.png`
  - 초록 화살표: HIT, 회색: MISS
  - 빨간 점: 교차점, 파란 화살표: 법선 벡터

#### ✅ Step 1.3: 스넬의 법칙 기반 굴절 계산 (WGSL)
- [x] 입사 벡터, 법선 벡터, 굴절률(n) 입력
- [x] 굴절 벡터 계산 (스넬의 법칙 벡터 형식)
- [x] 전반사(Total Internal Reflection) 감지 및 처리
- [x] 반사 벡터 계산 (TIR 및 Fresnel용)
- [x] 검증: Air (n=1.0) ↔ Glass (n=1.5) 테스트
  - 입사: 0°, 20°, 30°, 45°, 60°
  - 출사: 20°, 30°, 41.8° (임계각), 50° (TIR), 60° (TIR)
- [x] **Enhanced 시각화**: Sebastian Lague 스타일
  - 출력: `output/step_1_3_refraction.png` (1920x1080)
  - 주황색 광선: 입사광
  - 청록색 광선: 굴절광
  - 분홍/빨강 광선: 전반사
  - 회색 점선: 표면 법선
  - 초록 호: 입사각/굴절각 표시
  - 배경 색상: 매질 구분 (하늘색=Air, 연한 파랑=Glass)
  - 두꺼운 선, 대시 선, 호 그리기 기능 추가

#### 🔜 Step 1.4: 내부 반사 및 재굴절 경로 추적 (WGSL)
- [ ] 1차 내부 반사까지 광선 경로 추적
- [ ] 각 교차점 및 굴절/반사 벡터 저장
- [ ] 검증: 광선 경로 데이터를 이미지(또는 SVG)로 시각화

#### ✅ Step 1.5: 파장별 굴절률 (Drude-Lorentz)
- [x] ε∞ = 2.4, ωₚ = 1.37×10¹⁶ rad/s, γ = 4.0×10¹³ rad/s 적용
- [x] Lorentz oscillator 3개(78nm/160nm/240nm)로 bound-electron 공명 모델링
- [x] Johnson & Christy (1974) UV 데이터와 비교 (188-199nm 구간)
- [x] 검증: `examples/step_1_5_drude_steel.rs` → `output/step_1_5_drude_steel.png`
  - λ=100nm에서 n=1.38, k=0.22 → 공기 대비 n>1 확보
  - UV에서 k ≪ 가시광 영역(500nm) → 투과 가능성 확인

#### ✅ Step 1.6: 흡수 계산 (Beer-Lambert)
- [x] GPU path trace (Air→Steel, b/R=0.7)로 내부 경로 3.44 μm 확보 (기본 R=1.0 μm)
- [x] I = I₀ × exp(-αd) with α = 4πk/λ (k from Drude-Lorentz model)
- [x] 100-400 nm UV 스펙트럼 샘플링, 로그 스케일 transmittance 곡선 렌더링
- [x] 결과: `output/step_1_6_absorption.png` (GPU: Apple M3), R=1 μm일 때 전 파장대 T < 10⁻⁶ → 실질적으로 불투명
- 💡 Insight: 투과를 보려면 액적 반경을 UV 파장(50-100 nm) 수준으로 줄여야 함 (d ∝ R)
- 🛠️ 명령: `cargo run --example step_1_6_absorption -- 0.1` (0.1 μm), `-- 0.05` (0.05 μm) 등으로 파라미터 스윕

#### ✅ Step 1.7: 다중 파장 시뮬레이션 (UV Rainbow)
- [x] UV 스펙트럼 샘플링 (145-200 nm, n > 1 영역)
- [x] 각 파장에 대해 n(λ) 기반 경로 추적 실행
- [x] 출사각 계산 → 파장별 각도 분산 확인
- [x] 결과: `output/step_1_7_multiwave.png` (GPU: Apple M3)
- 🎯 **최적 파라미터**:
  - 액적 크기: 30 nm (투과율 최적화)
  - 파장 범위: 145-200 nm (플라즈마 공명 회피)
  - Impact parameter: b/R = 0.6
- 📊 **UV 무지개 발견**:
  - 각도 분산: 45.88° (rainbow width)
  - 투과율: 최대 0.13%, 평균 0.054%
  - n(λ) 변화: 1.144 → 1.483
  - 25/25 파장 성공 (100% 성공률)
- 💡 결론: **물리적으로 타당한 UV 무지개 확인!**

#### 🔜 Step 1.8: 대규모 광선 병렬 처리
- [ ] 수백만 개의 광선 시뮬레이션
- [ ] GPU 워크그룹 크기 최적화
- [ ] 검증: 성능 측정 및 결과 통계

#### 🔜 Step 1.9: 최종 출력 (스펙트로그램)
- [ ] 2D 히트맵: 각도(x축) vs 파장(y축)
- [ ] 색상: 강도(intensity)
- [ ] PNG 이미지 저장
- [ ] 검증: Phase 1 완료, 물리 로직 검증 성공

---

## Phase 2: 3D GPU 렌더링 (branch: `phase2-3d-render`)

### 목표
- Phase 1 물리 로직을 3D로 확장
- 실시간 비주얼라이제이션
- 수십억 개의 광선 실시간 처리

### 계획 (세부 단계는 Phase 1 완료 후 작성)
- 3D 광선 추적 (구면 좌표계)
- 카메라 시스템
- 실시간 렌더링 파이프라인
- 인터랙티브 파라미터 조정

---

## Phase 3: 물리 모델 검증 (branch: `phase3-validation`)

### 목표
- Mie 산란(Mie Scattering) 이론과 비교
- PyMieScatt 라이브러리 사용
- Ray Tracing 결과와 Mie 계산 결과의 경향성 비교

### 계획 (세부 단계는 Phase 2 완료 후 작성)
- Python 환경 설정
- PyMieScatt 적분 계산
- 결과 비교 분석 및 보고서 작성

---

## 📊 현재 상태

- **현재 Phase**: Phase 1
- **현재 Step**: Step 1.7 완료 ✅ (UV Rainbow 시뮬레이션)
- **다음 Step**: Step 1.8 (대규모 광선 병렬 처리)
- **현재 Branch**: `phase1-2d-logic`
- **GPU**: Apple M3
- **프로젝트 구조**: 전문적인 모듈 시스템 완성
- **시각화 스타일**: Sebastian Lague 스타일 (교육적, 직관적)
- **마지막 검증** (Step 1.7):
  - **UV 무지개 발견!** 각도 분산 45.88°, 투과율 0.054% (평균)
  - 최적 파라미터: 30nm 액적, 145-200nm 파장, b/R=0.6
  - Drude-Lorentz 모델: n(λ) = 1.144~1.483 (분산 확인)
  - 100% 성공률 (25/25 파장)
  - 출력: `output/step_1_7_multiwave.png`
- **이전 검증**:
  - Step 1.5: Drude-Lorentz 모델, Johnson & Christy 데이터 일치
  - Step 1.6: Beer-Lambert 흡수, 나노스케일 액적 필요성 확인
  - Step 1.3: 스넬의 법칙, 전반사 감지 (임계각 41.8°)

---

## 🔬 핵심 물리 공식

### 스넬의 법칙
```
n₁ sin(θ₁) = n₂ sin(θ₂)
```

### 굴절 벡터 계산
```
r = η · i + (η · cos(θ₁) - cos(θ₂)) · n
```
여기서:
- r: 굴절 벡터
- i: 입사 벡터
- n: 법선 벡터
- η = n₁/n₂

### 비어-램버트 법칙
```
I = I₀ × exp(-α × d)
```
여기서:
- I: 투과 강도
- I₀: 입사 강도
- α: 흡수 계수 (k와 관련)
- d: 경로 길이

### Drude-Lorentz 모델 (금속 광학 상수)
```
ε(ω) = ε∞ - \frac{ωₚ²}{ω² + iγω} + \sum_j \frac{f_j ωₚ²}{ω_j² - ω² - iΓ_j ω}
n(ω) + ik(ω) = √ε(ω)
```

---

## 📝 개발 원칙

1. **Micro-step 접근**: 각 단계를 매우 작게 나누어 구현
2. **단계별 검증**: 모든 단계마다 실행 가능한 결과물 출력
3. **순차 진행**: 이전 단계 검증 완료 후 다음 단계 진행
4. **명시적 승인**: 사용자가 "다음 단계로 진행하세요"라고 말할 때만 진행
5. **Git 브랜치 분리**: Phase별 독립적인 브랜치 관리

---

## 🎨 시각화 개선 사항 (Step 1.2 → Step 1.3)

### 기존 (Step 1.2)
- 기본 선 그리기
- 단순 색상 (빨강, 초록, 파랑)
- 흰 배경
- 화살표와 점만 표시

### Enhanced (Step 1.3) - Sebastian Lague Style
- **두꺼운 선** (`draw_thick_line`): 광선 가시성 향상
- **점선** (`draw_dashed_line`): 법선 벡터 구분
- **호** (`draw_arc`): 각도 시각화
- **배경 색상 구분**: 매질별 다른 배경 (Air=하늘색, Glass=파랑)
- **일관된 색상 팔레트**:
  - 주황색(#FFB400): 입사광
  - 청록색(#00B4FF): 굴절광
  - 분홍/빨강(#FF3264): 전반사
  - 회색 점선: 법선
  - 초록 호: 각도
- **고해상도**: 1920x1080
- **교육적 레이아웃**: 왼쪽=입사, 오른쪽=출사, 매질 구분 명확

### 추가된 Renderer2D 기능
```rust
draw_thick_line()       // 두께 조절 가능한 선
draw_dashed_line()      // 점선 (dash_length 조절)
draw_arc()              // 호 (각도 시각화)
fill_rect()             // 사각형 채우기 (배경)
draw_label()            // 텍스트 마커 (향후 확장)
```

## 🎓 참고 자료

- wgpu 문서: https://wgpu.rs/
- WGSL 스펙: https://www.w3.org/TR/WGSL/
- 광학 상수 데이터베이스: https://refractiveindex.info/
- PyMieScatt: https://pymiescatt.readthedocs.io/
- Sebastian Lague (유튜브): 교육적 시각화 스타일 참고
