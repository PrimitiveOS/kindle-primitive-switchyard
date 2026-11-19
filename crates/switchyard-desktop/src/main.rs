use std::{
    fs,
    path::{Path, PathBuf},
};

use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};
use switchyard_core::Outcome;
use switchyard_render::View;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&output)?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut app = View::preview();
    assert_eq!(app.input(Input::Tap(630.0, 1420.0)), Change::Partial);
    assert_eq!(app.outcome(), Some(Outcome::Buffer));
    assert_eq!(app.input(Input::Tap(470.0, 1020.0)), Change::Partial);
    assert_eq!(app.input(Input::Tap(630.0, 1420.0)), Change::Partial);
    assert_eq!(app.outcome(), Some(Outcome::Delivered));
    assert_eq!(app.game.completed(), 1);
    write_png(
        &output.join("switchyard-delivered.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    for _ in 0..5 {
        assert_eq!(app.input(Input::PageForward), Change::Full);
    }
    write_png(
        &output.join("switchyard.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(1115.0, 90.0)), Change::Exit);
    Ok(())
}
