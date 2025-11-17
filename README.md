# Iron Rainbow Simulator 🌈

**Physically-accurate simulation and visualization of rainbows formed by liquid steel nanoparticles in the UV spectrum**

![Iron Rainbow](https://img.shields.io/badge/Status-Complete-success)
![GPU](https://img.shields.io/badge/GPU-Apple%20M3-blue)
![Language](https://img.shields.io/badge/Language-Rust-orange)

---

## 🎯 Overview

The **Iron Rainbow Simulator** is a physics-based GPU-accelerated tool that simulates and visualizes the unique optical phenomenon of rainbows created by **liquid steel nanoparticles** in the ultraviolet (UV) spectrum.

Unlike traditional water-based rainbows, these "iron rainbows" occur in the **145-200 nm UV range** and require **30-nanometer droplets** to overcome strong metallic absorption.

### Key Discovery

> **UV rainbows from iron nanoparticles are physically possible!**
>
> Through Drude-Lorentz modeling and Beer-Lambert absorption calculations, we discovered that 30nm liquid steel droplets can produce observable rainbow scattering at 145-200nm wavelengths with ~0.13% peak transmittance.

---

## ✨ Features

### 🔬 Physics Engine
- **Drude-Lorentz dispersion model** for wavelength-dependent refractive index n(λ)
- **Beer-Lambert absorption** with wavelength-dependent extinction k(λ)
- **Snell's law** for refraction and total internal reflection
- **GPU-accelerated ray tracing** (690M rays/sec on Apple M3)

### 🎨 Real-Time 3D Visualization
- **Interactive 3D viewer** with FPS camera controls
- **Procedural space environment** with stars and ground
- **Phase function-based rendering** (LUT p(θ,λ) lookup)
- **Depth-based occlusion** for natural rainbow appearance
- **60 FPS** real-time performance

### 📊 Scientific Analysis
- **2D spectrogram generation** (angle vs wavelength heatmap)
- **Parallel ray tracing** for statistical analysis
- **Configurable LUT generation** for different parameters

---

## 🚀 Quick Start

### Prerequisites
- **Rust** (1.70+)
- **GPU** with wgpu support (Metal/Vulkan/DirectX 12)

### Installation

```bash
git clone https://github.com/winner-choichi/iron-rainbow.git
cd iron-rainbow
cargo build --release
```

### Run the 3D Viewer

```bash
cargo run --bin viewer --release
```

### Controls

#### Camera
- **WASD** - Move camera
- **QE** - Zoom in/out
- **Mouse** - Rotate view (click to capture, ESC to release)
- **R** - Reset camera

#### Sun Control
- **↑↓** - Sun elevation (-90° to 90°)
- **←→** - Sun azimuth (0° to 360°)

#### Rendering
- **+/-** - Adjust exposure
- **0-9** - Debug modes
- **P** - Print status
- **Q** - Quit

---

## 📁 Project Structure

```
iron-rainbow/
├── src/
│   ├── bin/
│   │   ├── lut_generator.rs    # LUT generation tool
│   │   └── viewer.rs            # 3D viewer application
│   ├── gpu/                     # GPU context & pipelines
│   ├── physics/                 # Optical physics (Drude, Beer-Lambert)
│   ├── geometry/                # Ray & circle primitives
│   ├── shaders/                 # WGSL shaders
│   │   ├── path_trace.wgsl     # Ray tracing shader
│   │   └── rainbow_phase.wgsl  # Phase function renderer
│   └── visualization/           # 2D rendering utilities
├── configs/
│   └── lut_config.toml          # LUT generation parameters
├── examples/                    # Step-by-step examples
└── output/                      # Generated images & data
```

---

## 🔬 Technical Details

### Phase 1: 2D Physics Validation

Validated the core physics using 2D GPU ray tracing:

| Step | Description | Performance |
|------|-------------|-------------|
| 1.5 | Drude-Lorentz model | n(100nm) = 1.38, k = 0.22 |
| 1.6 | Beer-Lambert absorption | R=30nm optimal |
| 1.7 | UV rainbow discovery | 145-200nm, Δθ=45.88° |
| 1.8 | Parallel ray tracing | 6.9M rays/sec |
| 1.9 | Spectrogram generation | 500K rays/sec |

**Result**: Confirmed UV rainbow feasibility with 30nm droplets at 145-200nm.

### Phase 2: 3D Real-Time Rendering

Built an interactive 3D visualization using phase function approach:

| Component | Technology | Details |
|-----------|------------|---------|
| LUT Generation | wgpu compute | 200K rays/wavelength × 64 wavelengths |
| Rendering | wgpu render | Phase function p(θ,λ) lookup |
| Scene | Procedural | Stars + ground + depth ordering |
| Performance | Apple M3 | 60 FPS @ 1280×720 |

**Result**: Real-time interactive UV rainbow visualization in space.

---

## 🎨 Visualization Examples

### Spectrogram (Phase 1)
2D heatmap showing intensity distribution across scattering angles and wavelengths.
- **X-axis**: Scattering angle (-180° to -120°)
- **Y-axis**: Wavelength (145-200 nm)
- **Color**: Intensity (blue → red)

### 3D Viewer (Phase 2)
Real-time interactive visualization with:
- Black space background with procedural stars
- Solid ground surface with depth-based occlusion
- Subtle UV rainbow arc (exposure: 0.01)
- Channel mapping: R=190nm, G=170nm, B=160nm

---

## ⚙️ Configuration

### LUT Generation (`configs/lut_config.toml`)

```toml
[grid]
wavelength_min_nm = 145.0
wavelength_max_nm = 200.0
angle_min_deg = -180.0
angle_max_deg = -120.0

[intensity]
normalize_mode = "global"
exposure = 0.01

[droplet]
radius_um = 0.03  # 30 nanometers
rays_per_wavelength = 200000

[renderer]
channel_wavelengths = [190.0, 170.0, 160.0]  # R, G, B
```

---

## 📚 Physical Model

### Drude-Lorentz Dispersion

```
ε(ω) = ε∞ - ωₚ²/(ω² + iγω) + Σⱼ fⱼωₚ²/(ωⱼ² - ω² - iΓⱼω)
n + ik = √ε(ω)
```

**Parameters** (Iron):
- ε∞ = 2.4
- ωₚ = 1.37×10¹⁶ rad/s
- γ = 4.0×10¹³ rad/s
- Lorentz oscillators at 78nm, 160nm, 240nm

### Beer-Lambert Absorption

```
I = I₀ × exp(-4πkd/λ)
```

Where:
- **k**: Extinction coefficient (from Drude-Lorentz)
- **d**: Path length inside droplet
- **λ**: Wavelength

### Rainbow Formation

1. **Refraction** at droplet surface (Snell's law)
2. **Internal reflection** (1 bounce)
3. **Dispersion** via wavelength-dependent n(λ)
4. **Absorption** via Beer-Lambert law
5. **Scattering** at ~138° (primary rainbow angle)

---

## 🛠️ Development

### Generate LUT

```bash
cargo run --bin lut_generator --release
```

Output:
- `output/iron_rainbow_lut.png` (512×64 grayscale)
- `output/iron_rainbow_lut.json` (metadata)

### Run Examples

```bash
cargo run --example step_1_7_multiwave --release
cargo run --example step_1_9_spectrogram --release
```

### Debug Modes (Viewer)

- **0**: Normal rendering
- **1**: R channel only
- **2**: G channel only
- **3**: B channel only
- **5**: LUT range check
- **7**: Scattering angle visualization

---

## 📖 References

- **Drude-Lorentz Model**: Johnson & Christy (1974) - Optical constants of iron
- **Rainbow Physics**: "Physically-Based Simulation of Rainbows" (SIGGRAPH 2012)
- **wgpu**: https://wgpu.rs/
- **Rust**: https://www.rust-lang.org/

---

## 📄 License

MIT License - See LICENSE file for details

---

## 👥 Contributors

- **Choe Chiwon** - Initial work and physics implementation

---

## 🙏 Acknowledgments

- Johnson & Christy for Fe optical constants data
- SIGGRAPH 2012 paper for rainbow rendering inspiration
- Rust & wgpu communities for excellent tools

---

**Made with ❤️ and GPU acceleration** ⚡🌈
