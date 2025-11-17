## Step 2.3 실행 방법

```bash
# 기본 실행 (main에서 기본 LUT 사용)
cargo run

# Viewer 바이너리 직접 실행 (기본 LUT)
cargo run --bin viewer

# 디버그 LUT로 실행
cargo run --bin viewer -- configs/lut_config_debug.toml
```

### 키보드 컨트롤
- **SPACE**: 애니메이션 on/off
- **↑/↓**: 태양 각도 조정
- **←/→**: 태양 방위 각도 조정
- **W/A/S/D**: 카메라 수평 이동
- **Q/E**: 카메라 상하 이동
- **마우스 이동**: 카메라 회전
- **+/-**: Exposure 조정
- **0-3**: 디버그 모드 (0=전체, 1=R채널, 2=G채널, 3=B채널)
- **P**: 현재 상태 출력
- **R**: 카메라 초기화
- **Q (키)**: 종료
