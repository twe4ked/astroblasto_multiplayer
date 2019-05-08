use crate::{Point2, Vector2};
use ggez::{graphics, nalgebra as na, Context};

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
}
