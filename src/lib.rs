use ggez::{
    audio::{self, SoundSource},
    event::{EventHandler, KeyCode, KeyMods},
    graphics, nalgebra as na, timer, Context, GameResult,
};

type Point2 = nalgebra::Point2<f32>;
type Vector2 = nalgebra::Vector2<f32>;

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

// An Actor is anything in the game world. We're not *quite* making a real entity-component system
// but it's pretty close. For a more complicated game you would want a real ECS, but for this it's
// enough to say that all our game objects contain pretty much the same data.
#[derive(Debug)]
enum ActorType {
    Player,
    Rock,
    Shot,
}

#[derive(Debug)]
struct Actor {
    tag: ActorType,
    pos: Point2,
    facing: f32,
    velocity: Vector2,
    ang_vel: f32,
    bbox_size: f32,

    // Lazily overload "life" with a double meaning: for shots, it is the time left to live, for
    // players and rocks, it is the actual hit points.
    life: f32,
}

const PLAYER_LIFE: f32 = 1.0;
const SHOT_LIFE: f32 = 2.0;
const ROCK_LIFE: f32 = 1.0;

const PLAYER_BBOX: f32 = 12.0;
const ROCK_BBOX: f32 = 12.0;
const SHOT_BBOX: f32 = 6.0;

const MAX_ROCK_VEL: f32 = 50.0;

fn create_player() -> Actor {
    Actor {
        tag: ActorType::Player,
        pos: Point2::origin(),
        facing: 0.,
        velocity: na::zero(),
        ang_vel: 0.,
        bbox_size: PLAYER_BBOX,
        life: PLAYER_LIFE,
    }
}

fn create_rock() -> Actor {
    Actor {
        tag: ActorType::Rock,
        pos: Point2::origin(),
        facing: 0.,
        velocity: na::zero(),
        ang_vel: 0.,
        bbox_size: ROCK_BBOX,
        life: ROCK_LIFE,
    }
}

fn create_shot() -> Actor {
    Actor {
        tag: ActorType::Shot,
        pos: Point2::origin(),
        facing: 0.,
        velocity: na::zero(),
        ang_vel: SHOT_ANG_VEL,
        bbox_size: SHOT_BBOX,
        life: SHOT_LIFE,
    }
}

/// Create the given number of rocks. Makes sure that none of them are within the given exclusion
/// zone (nominally the player). Note that this *could* create rocks outside the bounds of the
/// playing field, so it should be called before `wrap_actor_position()` happens.
fn create_rocks(num: i32, exclusion: Point2, min_radius: f32, max_radius: f32) -> Vec<Actor> {
    assert!(max_radius > min_radius);
    let new_rock = |_| {
        let mut rock = create_rock();
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
const SHOT_ANG_VEL: f32 = 0.1;

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

const MAX_PHYSICS_VEL: f32 = 250.0;

fn update_actor_position(actor: &mut Actor, dt: f32) {
    // Clamp the velocity to the max efficiently.
    let norm_sq = actor.velocity.norm_squared();
    if norm_sq > MAX_PHYSICS_VEL.powi(2) {
        actor.velocity = actor.velocity / norm_sq.sqrt() * MAX_PHYSICS_VEL;
    }
    let dv = actor.velocity * (dt);
    actor.pos += dv;
    actor.facing += actor.ang_vel;
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

/// Translates the world coordinate system, which has Y pointing up and the origin at the center,
/// to the screen coordinate system, which has Y pointing downward and the origin at the top-left.
fn world_to_screen_coords(screen_width: f32, screen_height: f32, point: Point2) -> Point2 {
    let x = point.x + screen_width / 2.0;
    let y = screen_height - (point.y + screen_height / 2.0);
    Point2::new(x, y)
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
        let font = graphics::Font::new(ctx, "/DejaVuSansMono.ttf")?;

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
}

impl MainState {
    pub fn new(ctx: &mut Context) -> GameResult<MainState> {
        println!("Game resource path: {:?}", ctx.filesystem);

        print_instructions();

        let assets = Assets::new(ctx)?;
        // let score_disp = graphics::Text::new(ctx, "score", &assets.font)?;
        // let level_disp = graphics::Text::new(ctx, "level", &assets.font)?;

        let player = create_player();
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
        };

        Ok(s)
    }

    fn fire_player_shot(&mut self) {
        self.player_shot_timeout = PLAYER_SHOT_TIME;

        let player = &self.player;
        let mut shot = create_shot();
        shot.pos = player.pos;
        shot.facing = player.facing;
        let direction = vec_from_angle(shot.facing);
        shot.velocity.x = SHOT_SPEED * direction.x;
        shot.velocity.y = SHOT_SPEED * direction.y;

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

    // fn update_ui(&mut self, ctx: &mut Context) {
    //     let score_str = format!("Score: {}", self.score);
    //     let level_str = format!("Level: {}", self.level);
    //     let score_text = graphics::Text::new(ctx, &score_str, &self.assets.font).unwrap();
    //     let level_text = graphics::Text::new(ctx, &level_str, &self.assets.font).unwrap();

    //     self.score_display = score_text;
    //     self.level_display = level_text;
    // }
}

fn print_instructions() {
    println!();
    println!("Welcome to ASTROBLASTO!");
    println!();
    println!("How to play:");
    println!("L/R arrow keys rotate your ship, up thrusts, space bar fires");
    println!();
}

fn draw_actor(ctx: &mut Context, actor: &Actor, world_coords: (f32, f32)) -> GameResult {
    let (screen_w, screen_h) = world_coords;
    let pos = world_to_screen_coords(screen_w, screen_h, actor.pos);
    let drawparams = graphics::DrawParam::new()
        .dest(pos)
        .rotation(actor.facing as f32)
        .offset(Point2::new(0.5, 0.5));
    let mut mesh: graphics::Mesh;

    match actor.tag {
        ActorType::Player => {
            mesh = graphics::Mesh::new_polygon(
                ctx,
                graphics::DrawMode::stroke(1.0),
                &[
                    na::Point2::new(0.0, -10.0),
                    na::Point2::new(8.0, 10.0),
                    na::Point2::new(0.0, 8.0),
                    na::Point2::new(-8.0, 10.0),
                ],
                graphics::WHITE,
            )
            .unwrap();
        }
        ActorType::Rock => {
            mesh = graphics::Mesh::new_polygon(
                ctx,
                graphics::DrawMode::stroke(1.0),
                &[
                    na::Point2::new(0.0, -10.0),
                    na::Point2::new(8.0, -2.0),
                    na::Point2::new(5.0, 10.0),
                    na::Point2::new(-5.0, 10.0),
                    na::Point2::new(-8.0, -2.0),
                ],
                graphics::WHITE,
            )
            .unwrap();
        }
        ActorType::Shot => {
            mesh = graphics::Mesh::new_polygon(
                ctx,
                graphics::DrawMode::stroke(1.0),
                &[
                    na::Point2::new(0.0, -5.0),
                    na::Point2::new(4.0, -1.0),
                    na::Point2::new(2.5, 5.0),
                    na::Point2::new(-2.5, 5.0),
                    na::Point2::new(-4.0, -1.0),
                ],
                graphics::WHITE,
            )
            .unwrap();
        }
    }

    graphics::draw(ctx, &mesh, drawparams)
}

/// Now we implement the `EventHandler` trait from `ggez::event`, which provides ggez with
/// callbacks for updating and drawing our game, as well as handling input events.
impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        const DESIRED_FPS: u32 = 60;

        while timer::check_update_time(ctx, DESIRED_FPS) {
            let seconds = 1.0 / (DESIRED_FPS as f32);

            // Update the player state based on the user input.
            player_handle_input(&mut self.player, &self.input, seconds);
            self.player_shot_timeout -= seconds;
            if self.input.fire && self.player_shot_timeout < 0.0 {
                self.fire_player_shot();
            }

            // Update the physics for all actors.
            update_actor_position(&mut self.player, seconds);
            wrap_actor_position(
                &mut self.player,
                self.screen_width as f32,
                self.screen_height as f32,
            );

            for act in &mut self.shots {
                update_actor_position(act, seconds);
                wrap_actor_position(act, self.screen_width as f32, self.screen_height as f32);
                handle_timed_life(act, seconds);
            }

            for act in &mut self.rocks {
                update_actor_position(act, seconds);
                wrap_actor_position(act, self.screen_width as f32, self.screen_height as f32);
            }

            // Handle the results of things moving:
            //
            // collision detection, object death, and if we have killed all the rocks in the level, spawn more of them.
            self.handle_collisions();
            self.clear_dead_stuff();
            self.check_for_level_respawn();

            // Finally we check for our end state.
            if self.player.life <= 0.0 {
                println!("Game over!");
                let _ = ggez::quit(ctx);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // Clear the screen...
        graphics::clear(ctx, graphics::BLACK);

        // Loop over all objects drawing them.
        {
            let coords = (self.screen_width, self.screen_height);

            let p = &self.player;
            draw_actor(ctx, p, coords)?;

            for s in &self.shots {
                draw_actor(ctx, s, coords)?;
            }

            for r in &self.rocks {
                draw_actor(ctx, r, coords)?;
            }
        }

        // Draw the GUI elements in the right places.
        let level_dest = Point2::new(10.0, 10.0);
        let score_dest = Point2::new(200.0, 10.0);

        let level_str = format!("Level: {}", self.level);
        let score_str = format!("Score: {}", self.score);
        let level_display = graphics::Text::new((level_str, self.assets.font, 32.0));
        let score_display = graphics::Text::new((score_str, self.assets.font, 32.0));
        graphics::draw(ctx, &level_display, (level_dest, 0.0, graphics::WHITE))?;
        graphics::draw(ctx, &score_display, (score_dest, 0.0, graphics::WHITE))?;

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

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymod: KeyMods) {
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
            _ => (),
        }
    }
}
