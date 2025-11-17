# Particle Cloud Design

## Overview
Replace single large droplet sphere with realistic particle cloud of tiny droplets.

## Physical Parameters

### From Phase 1
- **Droplet radius**: 30 nm (real physics)
- **Wavelength**: 145-200 nm (UV)
- **LUT**: Pre-computed rainbow intensities

### Visualization Scale
- **Real droplet**: 30 nm → **Visual droplet**: 0.05 m (50 cm)
- **Scale factor**: ~1.67 million (for visibility)
- **Box region**: 200m × 200m × 100m (W × H × D)

## Box Region Setup

### Position
- **Center**: [0, 0, 100] (100m in front of camera, towards +Z)
- **Dimensions**:
  - Width (X): 200m
  - Height (Y): 200m
  - Depth (Z): 100m (along sun direction)

### Sun Alignment
- Box placed at **anti-solar point** (opposite sun direction)
- Rainbow appears when looking through box towards anti-solar point

## Particle Distribution

### Number of Droplets
- **Option 1 (Low)**: 1,000 droplets (fast preview)
- **Option 2 (Medium)**: 10,000 droplets (good balance)
- **Option 3 (High)**: 100,000 droplets (realistic density)

### Distribution Pattern
- **Random uniform** within box volume
- Seed-based for reproducibility
- Option to add clustering/density variation

## Rendering Strategy

### GPU Data Structure
```rust
struct Droplet {
    position: [f32; 3],  // World position
    radius: f32,         // Visual radius (0.05m)
}
```

### Storage Buffer
- WGSL storage buffer (read-only)
- GPU accessible, no CPU update per frame

### Ray Marching Algorithm
```
For each pixel:
  1. Generate camera ray
  2. Find all droplets along ray path (sphere-ray intersection)
  3. Sort by distance
  4. March through each droplet:
     - Calculate anti-solar angle at sample point
     - Lookup LUT intensity
     - Accumulate color
  5. Blend accumulated color
```

### Optimization
- **Spatial partitioning**: Grid-based culling (optional, for 100k+)
- **Early termination**: Stop after opacity > 0.99
- **LOD**: Fewer samples for distant droplets

## Implementation Steps

1. **Create droplet generator**: Random positions in box
2. **WGSL storage buffer**: Upload droplet data to GPU
3. **Update fragment shader**: Multi-droplet ray marching
4. **Add UI controls**: Droplet count, box size, density

## Configuration Parameters

```toml
[particle_cloud]
droplet_count = 10000
droplet_visual_radius = 0.05  # meters
box_center = [0.0, 0.0, 100.0]
box_size = [200.0, 200.0, 100.0]
distribution = "uniform_random"
random_seed = 42
```

## Expected Result
- Realistic rainbow arc visible when looking through particle cloud
- Intensity falls off naturally at edges
- Multiple scattering effects (additive blending)
- Interactive: move sun → rainbow moves
