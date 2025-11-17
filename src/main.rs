use iron_rainbow::viewer_app;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("configs/lut_config.toml");
    viewer_app::run(Some(config_path))
}
