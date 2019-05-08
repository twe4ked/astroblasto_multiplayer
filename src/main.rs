//! An Asteroids-ish example game to show off ggez.
//! The idea is that this game is simple but still
//! non-trivial enough to be interesting.
use astroblasto_multiplayer::MainState;
use ggez::{conf, event, ContextBuilder, GameResult};
use std::env;
use std::path;

/// **********************************************************************
/// Finally our main function!  Which merely sets up a config and calls
/// `ggez::event::run()` with our `EventHandler` type.
/// **********************************************************************

fn main() -> GameResult {
    // We add the CARGO_MANIFEST_DIR/resources to the resource paths
    // so that ggez will look in our cargo project directory for files.
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ContextBuilder::new("astroblasto", "ggez")
        .window_setup(conf::WindowSetup::default().title("Astroblasto!"))
        .window_mode(conf::WindowMode::default().dimensions(640.0, 480.0))
        .add_resource_path(resource_dir);

    let (ctx, events_loop) = &mut cb.build()?;

    let game = &mut MainState::new(ctx)?;
    event::run(ctx, events_loop, game)
}
