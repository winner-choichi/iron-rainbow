# Repository Guidelines

## Project Structure & Module Organization
- Entry points live in `src/bin/viewer.rs` (interactive renderer) and `src/bin/lut_generator.rs` (dataset prep); reusable logic is split across `src/gpu`, `src/physics`, `src/simulation`, and `src/visualization`.
- `src/shaders/*.wgsl` mirrors the Rust pipeline modules; configs sit in `configs/`, runnable walkthroughs in `examples/`, and generated assets in the git-ignored `output/` tree.
- GPU-focused integration tests live in `tests/refraction_tests.rs`; prefer adding more cases there instead of writing ad-hoc binaries.

## Build, Test, and Development Commands
- `cargo check` — fast type/lint screen before editing shaders.
- `cargo run --bin viewer --release` — validates rendering and controls; add `WGPU_POWER_PREF=high` when debugging throttled GPUs.
- `cargo run --bin lut_generator --release` — produces `output/iron_rainbow_lut.{png,json}` for downstream tools.
- `cargo run --example step_1_9_spectrogram --release` — replays the documented physics milestone for regression images.
- `cargo test --all --features gpu-tests` — runs async `tokio` suites; skip the feature flag only if tests are CPU-bound.

## Coding Style & Naming Conventions
- Rust 2021 defaults apply: 4-space indentation, snake_case functions/modules, PascalCase structs/enums, SCREAMING_SNAKE_CASE constants.
- Always run `cargo fmt --all` followed by `cargo clippy --workspace --all-features -D warnings`; CI matches these settings.
- Keep shader entry points, bind group layouts, and Rust wrappers named consistently (`trace_stage` ↔ `TraceStage`) to avoid binding mismatches.

## Testing Guidelines
- Tests initialize `GpuContext` via `tokio::test`; lock down incident angles, wavelengths, and tolerances (<0.01 rad) for reproducibility.
- Use descriptive names like `test_snells_law_air_to_glass`; derive new expectations from physics formulas rather than recorded values.
- GPU-heavy tests should be gated with `#[cfg_attr(not(feature = "gpu-tests"), ignore)]` so CI can selectively enable them, and include reproduction steps in the PR.

## Commit & Pull Request Guidelines
- Follow the existing log: imperative sentence-case summaries under 72 characters with no trailing period (`Update LUT exposure`, `Add Snell tests`).
- Reference issue IDs and affected modules in the body, plus performance notes when GPU throughput changes.
- PRs should describe intent, list the commands executed (build/run/test), attach fresh screenshots or LUT snippets when visuals change, and mention the platform validated (e.g., Metal on Apple M3).

## GPU & Configuration Tips
- Keep `configs/lut_config.toml` wavelengths synced with the README’s UV ranges; stale values immediately desaturate the viewer and spectrogram examples.
- When editing shaders, update the paired Rust pipeline layouts in `src/gpu/pipelines` before running `cargo run`, or wgpu will panic with bind mismatches.
