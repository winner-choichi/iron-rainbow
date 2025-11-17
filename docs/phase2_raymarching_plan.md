# Phase 2 - 3D Ray Marching Implementation Plan

## 목표
LUT 기반 실시간 3D 무지개 렌더링 (Sebastian Lague 스타일)

## 아키텍처

### 1. 3D 공간 설정
```
Sky Dome (하늘 구)
  ↓
Droplet Region (물방울 영역, 구형)
  - Center: (0, 0, 0)
  - Radius: 100m
  ↓
Camera (관측자)
  - Position: (0, 0, -200)
  - Look-at: (0, 0, 0)
  ↓
Sun Direction (태양 방향)
  - Directional light
  - 사용자가 조정 가능
```

### 2. Ray Marching 알고리즘
```wgsl
for each pixel:
    1. Generate ray from camera
    2. Check intersection with droplet sphere
    3. If hit:
        March along ray inside sphere
        For each step:
            - Calculate anti-solar angle θ
              θ = acos(dot(-ray_dir, sun_dir))
            - Sample LUT[θ, wavelength]
            - Accumulate color
    4. Return final color
```

### 3. Anti-Solar Angle 계산
```
anti_solar_angle = angle between:
  - Ray direction (camera → point)
  - Sun direction
  
For rainbow: 42° (primary), 51° (secondary)
For Iron Rainbow (UV): LUT에서 조회
```

### 4. Camera Controls
- **WASD**: 카메라 이동
- **마우스**: 카메라 회전
- **↑↓**: 태양 고도 조정
- **←→**: 태양 방위각 조정

### 5. Uniform 구조 확장
```rust
struct ViewerUniform {
    // Camera
    camera_pos: vec3<f32>,
    camera_forward: vec3<f32>,
    camera_right: vec3<f32>,
    camera_up: vec3<f32>,
    
    // Sun
    sun_dir: vec3<f32>,
    
    // Droplet region
    droplet_center: vec3<f32>,
    droplet_radius: f32,
    
    // LUT params (기존)
    ...
}
```

## 참고 자료
- Sebastian Lague: Coding Adventure - Ray Marching
- Inigo Quilez: Ray Marching Distance Fields
- Scratchapixel: Ray Tracing - Sphere Intersection

## 구현 단계
1. ✅ 설계 문서 작성
2. [ ] ViewerUniform 확장 (3D 카메라, 태양)
3. [ ] WGSL: 카메라 ray 생성
4. [ ] WGSL: 구 교차 검사
5. [ ] WGSL: Ray marching 루프
6. [ ] WGSL: Anti-solar angle + LUT 조회
7. [ ] Rust: 카메라 컨트롤 (WASD + 마우스)
8. [ ] 테스트 및 검증
