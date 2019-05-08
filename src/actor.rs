use crate::{Point2, Vector2};
use ggez::{graphics, nalgebra as na, Context, GameResult};

const PLAYER_LIFE: f32 = 1.0;
const SHOT_LIFE: f32 = 2.0;
const ROCK_LIFE: f32 = 1.0;

const PLAYER_BBOX: f32 = 12.0;
const ROCK_BBOX: f32 = 12.0;
const SHOT_BBOX: f32 = 6.0;

const SHOT_ANG_VEL: f32 = 0.1;

// An Actor is anything in the game world. We're not *quite* making a real entity-component system
// but it's pretty close. For a more complicated game you would want a real ECS, but for this it's
// enough to say that all our game objects contain pretty much the same data.
#[derive(Debug)]
pub enum ActorType {
    Player,
    Rock,
    Shot,
}

#[derive(Debug)]
pub struct Actor {
    pub tag: ActorType,
    pub pos: Point2,
    pub facing: f32,
    pub velocity: Vector2,
    pub ang_vel: f32,
    pub bbox_size: f32,

    // Lazily overload "life" with a double meaning: for shots, it is the time left to live, for
    // players and rocks, it is the actual hit points.
    pub life: f32,
}

impl Actor {
    pub fn polygon(&self, ctx: &mut Context) -> graphics::Mesh {
        match self.tag {
            ActorType::Player => graphics::Mesh::new_polygon(
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
            .unwrap(),
            ActorType::Rock => graphics::Mesh::new_polygon(
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
            .unwrap(),
            ActorType::Shot => graphics::Mesh::new_polygon(
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
            .unwrap(),
        }
    }

    pub fn create_player() -> Self {
        Self {
            tag: ActorType::Player,
            pos: Point2::origin(),
            facing: 0.,
            velocity: na::zero(),
            ang_vel: 0.,
            bbox_size: PLAYER_BBOX,
            life: PLAYER_LIFE,
        }
    }

    pub fn create_rock() -> Self {
        Self {
            tag: ActorType::Rock,
            pos: Point2::origin(),
            facing: 0.,
            velocity: na::zero(),
            ang_vel: 0.,
            bbox_size: ROCK_BBOX,
            life: ROCK_LIFE,
        }
    }

    pub fn create_shot() -> Self {
        Self {
            tag: ActorType::Shot,
            pos: Point2::origin(),
            facing: 0.,
            velocity: na::zero(),
            ang_vel: SHOT_ANG_VEL,
            bbox_size: SHOT_BBOX,
            life: SHOT_LIFE,
        }
    }

    pub fn draw_actor(&self, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult {
        let (screen_w, screen_h) = world_coords;
        let pos = Self::world_to_screen_coords(screen_w, screen_h, self.pos);
        let drawparams = graphics::DrawParam::new()
            .dest(pos)
            .rotation(self.facing as f32)
            .offset(Point2::new(0.5, 0.5));
        let mesh = self.polygon(ctx);

        graphics::draw(ctx, &mesh, drawparams)
    }

    /// Translates the world coordinate system, which has Y pointing up and the origin at the center,
    /// to the screen coordinate system, which has Y pointing downward and the origin at the top-left.
    fn world_to_screen_coords(screen_width: f32, screen_height: f32, point: Point2) -> Point2 {
        let x = point.x + screen_width / 2.0;
        let y = screen_height - (point.y + screen_height / 2.0);
        Point2::new(x, y)
    }
}
