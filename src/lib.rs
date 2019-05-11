mod actor;

use actor::Actor;
use ggez::{
    audio::{self, SoundSource},
    event::{EventHandler, KeyCode, KeyMods},
    graphics, nalgebra as na, timer, Context, GameResult,
};

pub type Point2 = na::Point2<f32>;
pub type Vector2 = na::Vector2<f32>;

/// Create a unit vector representing the given angle (in radians).
fn vec_from_angle(angle: f32) -> Vector2 {
    let vx = angle.sin();
    let vy = angle.cos();
    Vector2::new(vx, vy)
}

/// Makes a random `Vector2` with the given max magnitude.
fn random_vec(max_magnitude: f32) -> Vector2 {
    let angle = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
    let mag = rand::random::<f32>() * max_magnitude;
    vec_from_angle(angle) * (mag)
}

const MAX_ROCK_VEL: f32 = 50.0;

/// Create the given number of rocks. Makes sure that none of them are within the given exclusion
/// zone (nominally the player). Note that this *could* create rocks outside the bounds of the
/// playing field, so it should be called before `wrap_actor_position()` happens.
fn create_rocks(num: i32, exclusion: Point2, min_radius: f32, max_radius: f32) -> Vec<Actor> {
    assert!(max_radius > min_radius);
    let new_rock = |_| {
        let mut rock = Actor::create_rock();
        let r_angle = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
        let r_distance = rand::random::<f32>() * (max_radius - min_radius) + min_radius;
        rock.pos = exclusion + vec_from_angle(r_angle) * r_distance;
        rock.velocity = random_vec(MAX_ROCK_VEL);
        rock
    };
    (0..num).map(new_rock).collect()
}

// Now we make functions to handle physics. We do simple Newtonian physics (so we do have
// inertia), and cap the max speed so that we don't have to worry too much about small objects
// clipping through each other.
//
// Our unit of world space is simply pixels, though we do transform the coordinate system so that
// +y is up and -y is down.

const SHOT_SPEED: f32 = 200.0;

// Acceleration in pixels per second.
const PLAYER_THRUST: f32 = 100.0;
// Rotation in radians per second.
const PLAYER_TURN_RATE: f32 = 3.0;
// Seconds between shots.
const PLAYER_SHOT_TIME: f32 = 0.5;

fn player_handle_input(actor: &mut Actor, input: &InputState, dt: f32) {
    actor.facing += dt * PLAYER_TURN_RATE * input.xaxis;

    if input.yaxis > 0.0 {
        player_thrust(actor, dt);
    }
}

fn player_thrust(actor: &mut Actor, dt: f32) {
    let direction_vector = vec_from_angle(actor.facing);
    let thrust_vector = direction_vector * (PLAYER_THRUST);
    actor.velocity += thrust_vector * (dt);
}

fn update_actor_position(actor: &mut Actor, dt: f32) {
    let dv = actor.velocity * (dt);
    actor.pos += dv;
    actor.facing += actor.ang_vel;
}

const MAX_PHYSICS_VEL: f32 = 250.0;

fn clamp_actor_velocity(actor: &mut Actor) {
    // Make sure players can't go too fast to get hectic.
    let norm_sq = actor.velocity.norm_squared();
    if norm_sq > MAX_PHYSICS_VEL.powi(2) {
        actor.velocity = actor.velocity / norm_sq.sqrt() * MAX_PHYSICS_VEL;
    }
}

/// Takes an actor and wraps its position to the bounds of the screen, so if it goes off the left
/// side of the screen it will re-enter on the right side and so on.
fn wrap_actor_position(actor: &mut Actor, sx: f32, sy: f32) {
    // Wrap screen.
    let screen_x_bounds = sx / 2.0;
    let screen_y_bounds = sy / 2.0;
    if actor.pos.x > screen_x_bounds {
        actor.pos.x -= sx;
    } else if actor.pos.x < -screen_x_bounds {
        actor.pos.x += sx;
    };
    if actor.pos.y > screen_y_bounds {
        actor.pos.y -= sy;
    } else if actor.pos.y < -screen_y_bounds {
        actor.pos.y += sy;
    }
}

fn handle_timed_life(actor: &mut Actor, dt: f32) {
    actor.life -= dt;
}

/// Translates the world coordinate system to coordinates suitable for the audio system.
fn world_to_audio_coords(screen_width: f32, screen_height: f32, point: Point2) -> [f32; 3] {
    let x = point.x * 2.0 / screen_width;
    let y = point.y * 2.0 / screen_height;
    let z = 0.0;
    [x, y, z]
}

/// A structure to contain the fonts, sounds, etc. that we need to hang on to; this is our "asset
/// management system".  All the file names and such are just hard-coded.
struct Assets {
    font: graphics::Font,
    shot_sound: audio::SpatialSource,
    hit_sound: audio::SpatialSource,
}

impl Assets {
    fn new(ctx: &mut Context) -> GameResult<Assets> {
        let font = graphics::Font::new(ctx, "/manaspc.ttf")?;

        let mut shot_sound = audio::SpatialSource::new(ctx, "/pew.ogg")?;
        let mut hit_sound = audio::SpatialSource::new(ctx, "/boom.ogg")?;

        shot_sound.set_ears([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        hit_sound.set_ears([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

        Ok(Assets {
            font,
            shot_sound,
            hit_sound,
        })
    }
}

/// The `InputState` is exactly what it sounds like, it just keeps track of the user's input state
/// so that we turn keyboard events into something state-based and device-independent.
#[derive(Debug)]
struct InputState {
    xaxis: f32,
    yaxis: f32,
    fire: bool,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            xaxis: 0.0,
            yaxis: 0.0,
            fire: false,
        }
    }
}

enum State {
    Instructions,
    Playing,
    Dead,
}

/// Now we're getting into the actual game loop. The `MainState` is our game's "global" state, it
/// keeps track of everything we need for actually running the game.
///
/// Our game objects are simply a vector for each actor type, and we probably mingle gameplay-state
/// (like score) and hardware-state (like `input`) a little more than we should, but for something
/// this small it hardly matters.
pub struct MainState {
    player: Actor,
    shots: Vec<Actor>,
    rocks: Vec<Actor>,
    level: i32,
    score: i32,
    assets: Assets,
    screen_width: f32,
    screen_height: f32,
    input: InputState,
    player_shot_timeout: f32,
    state: State,
    state_transition: f32,
}

impl MainState {
    pub fn new(ctx: &mut Context) -> GameResult<MainState> {
        let assets = Assets::new(ctx)?;
        let player = Actor::create_player();
        let rocks = create_rocks(5, player.pos, 100.0, 250.0);

        let s = MainState {
            player,
            shots: Vec::new(),
            rocks,
            level: 0,
            score: 0,
            assets,
            screen_width: ctx.conf.window_mode.width,
            screen_height: ctx.conf.window_mode.height,
            input: InputState::default(),
            player_shot_timeout: 0.0,
            state_transition: 5.0,
            state: State::Instructions,
        };

        Ok(s)
    }

    fn reset_state(&mut self) {
        let player = Actor::create_player();
        let rocks = create_rocks(5, player.pos, 100.0, 250.0);

        self.player = player;
        self.shots = Vec::new();
        self.rocks = rocks;
        self.level = 0;
        self.score = 0;
        self.player_shot_timeout = 0.0;
    }

    fn fire_player_shot(&mut self) {
        self.player_shot_timeout = PLAYER_SHOT_TIME;

        let player = &self.player;
        let mut shot = Actor::create_shot();
        shot.pos = player.pos;
        shot.facing = player.facing;
        shot.velocity = player.velocity;
        let direction = vec_from_angle(shot.facing);
        shot.velocity.x += SHOT_SPEED * direction.x;
        shot.velocity.y += SHOT_SPEED * direction.y;

        self.shots.push(shot);

        let pos = world_to_audio_coords(self.screen_width, self.screen_height, player.pos);
        self.assets.shot_sound.set_position(pos);
        let _ = self.assets.shot_sound.play();
    }

    fn clear_dead_stuff(&mut self) {
        self.shots.retain(|s| s.life > 0.0);
        self.rocks.retain(|r| r.life > 0.0);
    }

    fn handle_collisions(&mut self) {
        for rock in &mut self.rocks {
            let pdistance = rock.pos - self.player.pos;
            if pdistance.norm() < (self.player.bbox_size + rock.bbox_size) {
                self.player.life = 0.0;
            }
            for shot in &mut self.shots {
                let distance = shot.pos - rock.pos;
                if distance.norm() < (shot.bbox_size + rock.bbox_size) {
                    shot.life = 0.0;
                    rock.life = 0.0;
                    self.score += 1;

                    let pos =
                        world_to_audio_coords(self.screen_width, self.screen_height, rock.pos);
                    self.assets.shot_sound.set_position(pos);
                    let _ = self.assets.hit_sound.play();
                }
            }
        }
    }

    fn check_for_level_respawn(&mut self) {
        if self.rocks.is_empty() {
            self.level += 1;
            let r = create_rocks(self.level + 5, self.player.pos, 100.0, 250.0);
            self.rocks.extend(r);
        }
    }

    fn draw_ui(&mut self, ctx: &mut Context) -> GameResult {
        let level_dest = Point2::new(self.scaled_size(10.0), self.scaled_size(10.0));
        let score_dest = Point2::new(self.scaled_size(140.0), self.scaled_size(10.0));

        let level_str = format!("Level: {}", self.level);
        let score_str = format!("Score: {}", self.score);

        let level_display =
            graphics::Text::new((level_str, self.assets.font, self.scaled_size(20.0)));
        let score_display =
            graphics::Text::new((score_str, self.assets.font, self.scaled_size(20.0)));

        graphics::draw(ctx, &level_display, (level_dest, 0.0, graphics::WHITE))?;
        graphics::draw(ctx, &score_display, (score_dest, 0.0, graphics::WHITE))?;

        Ok(())
    }

    fn draw_instructions(&self, ctx: &mut Context) -> GameResult {
        let instructions = graphics::Text::new((
            String::from("\n   !!! Welcome to ASTROBLASTO!!!\n\n\nHow to play:\nL/R arrow keys rotate your ship,\nup thrusts, space bar fires"),
            self.assets.font,
            self.scaled_size(32.0),
        ));

        graphics::draw(
            ctx,
            &instructions,
            (Point2::new(50.0, 50.0), 0.0, graphics::WHITE),
        )?;

        Ok(())
    }

    fn draw_death_screen(&self, ctx: &mut Context) -> GameResult {
        let text = graphics::Text::new((String::from("You died!"), self.assets.font, 32.0));
        graphics::draw(ctx, &text, (Point2::new(10.0, 10.0), 0.0, graphics::WHITE))?;
        Ok(())
    }

    /// Takes a given size and scales it based on the window dimensions
    fn scaled_size(&self, size: f32) -> f32 {
        if self.screen_width > 800.0 {
            size * 2.0
        } else {
            size
        }
    }
}

/// Now we implement the `EventHandler` trait from `ggez::event`, which provides ggez with
/// callbacks for updating and drawing our game, as well as handling input events.
impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        const DESIRED_FPS: u32 = 60;

        while timer::check_update_time(ctx, DESIRED_FPS) {
            let delta = 1.0 / (DESIRED_FPS as f32);

            match self.state {
                State::Instructions => {
                    if self.state_transition >= 0.0 {
                        self.state_transition -= delta;
                    } else {
                        self.state = State::Playing;
                    }

                    if self.input.fire {
                        self.state = State::Playing;
                        self.input.fire = false;
                    }
                }
                State::Playing => {
                    // Update the player state based on the user input.
                    player_handle_input(&mut self.player, &self.input, delta);
                    self.player_shot_timeout -= delta;
                    if self.input.fire && self.player_shot_timeout < 0.0 {
                        self.fire_player_shot();
                    }

                    // Update the physics for all actors.
                    update_actor_position(&mut self.player, delta);
                    clamp_actor_velocity(&mut self.player);
                    wrap_actor_position(
                        &mut self.player,
                        self.screen_width as f32,
                        self.screen_height as f32,
                    );

                    for act in &mut self.shots {
                        update_actor_position(act, delta);
                        wrap_actor_position(
                            act,
                            self.screen_width as f32,
                            self.screen_height as f32,
                        );
                        handle_timed_life(act, delta);
                    }

                    for act in &mut self.rocks {
                        update_actor_position(act, delta);
                        wrap_actor_position(
                            act,
                            self.screen_width as f32,
                            self.screen_height as f32,
                        );
                    }

                    // Handle the results of things moving:
                    //
                    // collision detection, object death, and if we have killed all the rocks in
                    // the level, spawn more of them.
                    self.handle_collisions();
                    self.clear_dead_stuff();
                    self.check_for_level_respawn();

                    // Finally we check for our end state.
                    if self.player.life <= 0.0 {
                        self.state = State::Dead;
                        self.state_transition = 5.0;
                        self.reset_state();
                    }
                }
                State::Dead => {
                    if self.state_transition >= 0.0 {
                        self.state_transition -= delta;
                    } else {
                        self.state = State::Playing;
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // Clear the screen...
        graphics::clear(ctx, graphics::Color::new(0.0, 0.015, 0.1, 1.0));

        match self.state {
            State::Instructions => {
                self.draw_instructions(ctx)?;
            }
            State::Playing => {
                // Loop over all objects drawing them.
                let coords = (self.screen_width, self.screen_height);

                let p = &self.player;
                p.draw_actor(ctx, coords)?;

                for s in &self.shots {
                    s.draw_actor(ctx, coords)?;
                }

                for r in &self.rocks {
                    r.draw_actor(ctx, coords)?;
                }

                self.draw_ui(ctx)?;
            }
            State::Dead => {
                self.draw_death_screen(ctx)?;
            }
        }

        // Then we flip the screen.
        graphics::present(ctx)?;

        // Yield the timeslice.
        //
        // This tells the OS that we're done using the CPU but it should get back to this program
        // as soon as it can. This ideally prevents the game from using 100% CPU all the time even
        // if vsync is off. The actual behavior can be a little platform-specific.
        timer::yield_now();
        Ok(())
    }

    // Handle key events. These just map keyboard events and alter our input state appropriately.
    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::Up => {
                self.input.yaxis = 1.0;
            }
            KeyCode::Left => {
                self.input.xaxis = -1.0;
            }
            KeyCode::Right => {
                self.input.xaxis = 1.0;
            }
            KeyCode::Space => {
                self.input.fire = true;
            }
            KeyCode::P => {
                let img = graphics::screenshot(ctx).expect("Could not take screenshot");
                img.encode(ctx, graphics::ImageFormat::Png, "/screenshot.png")
                    .expect("Could not save screenshot");
            }
            KeyCode::Escape => ggez::quit(ctx),
            _ => (),
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymod: KeyMods) {
        match keycode {
            KeyCode::Up => {
                self.input.yaxis = 0.0;
            }
            KeyCode::Left | KeyCode::Right => {
                self.input.xaxis = 0.0;
            }
            KeyCode::Space => {
                self.input.fire = false;
            }
            KeyCode::Q => {
                let _ = ggez::quit(ctx);
            }
            _ => (),
        }
    }
}
