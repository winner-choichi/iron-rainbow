# Phase 2 Viewer Plan

## 목표
- LUT 텍스처(`iron_rainbow_lut.png`)와 메타데이터를 GPU에 업로드하여 실시간 3D/2D 뷰어로 시각화
- Anti-solar angle 계산과 LUT 조회만으로 화면에 무지개를 표시 (물리 연산 없음)

## 구성 요소
1. **wgpu + winit 윈도우 시스템**
   - 스왑체인, depth/stencil 필요 없음 (단순 full-screen quad)
   - 리사이즈/입력 처리
2. **리소스 로더**
   - LUT PNG → `ImageBuffer` → `wgpu::Texture`
   - LUT JSON → `LutMetadata` 구조체 → uniform 버퍼
3. **셰이더 (WGSL)**
   - vertex: full-screen triangle
   - fragment: anti-solar angle 계산 → LUT 샘플 → false-color 매핑 누적
4. **카메라/태양 파라미터**
   - 초기값: 관측자 at origin, 태양 고도/방위 입력값
   - Uniform 구조체: `camera_matrix`, `sun_direction`, `viewer_params`
5. **UI (추후)**
   - 파라미터 슬라이더 (태양 고도, LUT 선택)
   - FPS, LUT info 표시

## 렌더 패스 설계
1. CPU에서 anti-solar angle을 구하는 대신, 셰이더에서 화면 좌표 → view direction → angle 계산
2. 파장 루프는 fragment에서 fixed-step으로 수행 (물리 LUT 높이만큼)
3. False-color 매핑은 LUT 메타데이터에서 불러온 stop을 uniform 배열로 전달, fragment에서 선형 보간

## TODO
- [ ] winit + wgpu 창 생성, 기본 렌더 루프
- [ ] LUT/JSON 로더 및 uniform 버퍼 구축
- [ ] WGSL 셰이더 초안 작성 (full-screen triangle + LUT lookup)
- [ ] 간단한 키보드 입력(태양 고도 증가/감소) 처리
- [ ] HUD/텍스트는 Step 2.3에서 추가 예정
