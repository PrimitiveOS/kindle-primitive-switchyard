use std::path::PathBuf;

use switchyard_render::View;

fn main() -> Result<(), String> {
    let directory = std::env::var_os("PRIMITIVE_APP_DIR")
        .map_or_else(|| PathBuf::from("/mnt/us/switchyard"), PathBuf::from);
    primitive_app::run(View::open(directory.join("switchyard.json")), &directory)
}
