use iron_rainbow::viewer_app;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config_override = args.get(1).map(|s| s.as_str());
    viewer_app::run(config_override)
}
