# Phase 2 LUT Plan

## 🎯 목표
- Phase 1의 2D GPU 시뮬레이터를 확장하여 **파장 × 탈출각** 분포를 2D 텍스처(LUT)로 생성
- Phase 2의 실시간 3D 렌더러는 이 LUT만 조회하여 색상을 계산 (Sebastian Lague 방식)

## 📁 산출물
1. `output/iron_rainbow_lut.png` (또는 HDR) – intensity 텍스처
2. `output/iron_rainbow_lut.json` – 메타데이터 (좌표 범위, 정규화, false-color 규칙)
3. `configs/lut_config.toml` – LUT 생성 파라미터 입력 파일

## 🔧 파라미터 (lut_config.toml)
```toml
[grid]
wavelength_min_nm = 145.0
wavelength_max_nm = 200.0
wavelength_steps = 64
angle_min_deg = -180.0
angle_max_deg = -120.0
angle_steps = 512
angle_supersample = 4

[intensity]
normalize_mode = "per_wavelength"
exposure = 1.0

[droplet]
radius_um = 0.03
impact_min = 0.05
impact_max = 0.85
impact_samples = 128
rays_per_wavelength = 200000

[wavelength]
batch_size = 4
wavelength_step_nm = 0.5

[output]
texture_path = "output/iron_rainbow_lut.png"
metadata_path = "output/iron_rainbow_lut.json"
format = "png16"

[renderer]
false_color = [
  { wavelength = 145.0, color = [64, 64, 255] },
  { wavelength = 160.0, color = [64, 200, 255] },
  { wavelength = 175.0, color = [64, 255, 150] },
  { wavelength = 190.0, color = [255, 210, 64] },
  { wavelength = 200.0, color = [255, 120, 64] }
]
channel_wavelengths = [190.0, 170.0, 145.0] # Viewer R/G/B 채널에 대응
```

## 🧵 파이프라인
1. **LUT Generator (Step 2.0~2.1)**
   - `configs/lut_config.toml` 읽기 → 파장 리스트 생성
   - 각 파장에 대해 Step 1.8 path tracer를 실행, 탈출각 히스토그램 계산
   - intensity를 grid에 채우고 PNG/HDR로 저장 + 메타데이터 기록
2. **Real-time Renderer (Step 2.2~2.4)**
   - LUT 텍스처와 메타데이터를 wgpu로 로드
   - 셰이더는 anti-solar angle을 X좌표로 사용, Y축으로 채널 파장을 루프하며 false-color 매핑
   - UI(HUD)에서 LUT 정보, 태양/카메라 파라미터를 표시하고 즉시 피드백 제공

## ✅ Step 2.0 Deliverables
- 설정 파일 `configs/lut_config.toml`
- 설계 문서 `docs/phase2_lut_plan.md`
- 시각화 예제 `output/step_2_0_lut_layout.png` (LUT 좌표계를 도식화)
- 콘솔 리포트: 파장/각도 축, 해상도, false-color 샘플 테이블

## ✅ Step 2.1 Deliverables
- CLI: `cargo run --bin lut_generator -- [config]`
- 기본 config: `configs/lut_config.toml` (전체 범위)
- 빠른 검증용 config: `configs/lut_config_debug.toml`
- 출력: `output/iron_rainbow_lut.png` + `output/iron_rainbow_lut.json`
- 메타데이터는 각 축 범위, 정규화 모드, false-color 스톱/채널 파장 정보를 포함

## ✅ Step 2.2 Deliverables
- 뷰어 실행 통합: `viewer_app::run()` → `cargo run`
- 마우스/키보드 컨트롤, 디버그 모드(0~9), HUD/상태 출력(`P`)
- LUT 채널 지정(`channel_wavelengths`), half-texel 샘플링

## TODO (Step 2.3 이후)
- UI/HUD(imgui)로 태양/카메라 파라미터 조정
- 스크린샷, LUT 정규화 튜닝, 다중 LUT 로딩 지원

