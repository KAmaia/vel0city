use bsp;
use bsp::Plane;
use bsp::PlaneCollisionVisitor;
use bsp::cast::CastResult;
use player::{
    PLAYER_ONGROUND
};
use na;
use Game;

pub struct MoveInput {
    /// The velocity the player "wishes" to have 
    pub wishvel: na::Vec3<f32>,

    pub jump: bool,
}

pub fn move_player(game: &mut Game, playeridx: u32, input: &MoveInput, dt: f32) {
    {
        let pl = &mut game.players[playeridx as usize];

        let accel = if pl.flags.contains(PLAYER_ONGROUND) {
            game.movesettings.accel
        } else {
            game.movesettings.airaccel
        };

        let friction = if pl.flags.contains(PLAYER_ONGROUND) {
            game.movesettings.friction 
        } else {
            0.0
        };

        let horizvel = na::Vec3::new(pl.vel.x, 0.0, pl.vel.z);
        let wishspeed = na::clamp(na::norm(&input.wishvel), 0.0, game.movesettings.movespeed);
        if !na::approx_eq(&wishspeed, &0.0) { 
            let movedir = na::normalize(&input.wishvel);

            let curspeed = na::dot(&horizvel, &movedir); 
            let maxdelta = accel * wishspeed * dt;
            let addspeed = na::clamp((wishspeed - curspeed), -maxdelta, maxdelta);
            pl.vel = pl.vel + (movedir * addspeed);
        }

        let speed = na::norm(&pl.vel);
        if !na::approx_eq(&speed, &0.0) {
            let dir = na::normalize(&pl.vel);
            let removespeed = friction * dt * if speed < game.movesettings.speedeps {
                game.movesettings.speedeps
            } else {
                speed
            };

            let newspeed = na::clamp(speed - removespeed, 0.0, game.movesettings.maxspeed); 

            pl.vel = dir * newspeed;
        }

        pl.vel.y -= game.movesettings.gravity * dt;

        if input.jump && pl.flags.contains(PLAYER_ONGROUND) {
            pl.vel.y = game.movesettings.jumpspeed;
            pl.flags.remove(PLAYER_ONGROUND);
        }

        let mut dt = dt;
        let mut hit_floor = false;
        for _ in 0..3 {
            let moveray = bsp::cast::Ray {
                orig: pl.pos,
                dir: pl.vel * dt,
                halfextents: pl.halfextents
            };

            let mut vis = ClipMoveVisitor { 
                best: None,
                vel: pl.vel,
                curvel: pl.vel,
                pos: pl.pos.to_vec(),
                hit_floor: false 
            };

            game.map.bsp.cast_ray_visitor(&moveray, &mut vis);

            if let Some(bsp::cast::CastResult { toi, .. }) = vis.best {
                pl.pos = pl.pos + (pl.vel * dt * toi);
                dt = dt * (1.0 - toi);
                if vis.hit_floor {
                    hit_floor = true;
                }
            } else {
                pl.pos = pl.pos + pl.vel * dt;
                break;
            }
            pl.vel = vis.curvel;
        }
        if hit_floor {
            pl.flags.insert(PLAYER_ONGROUND)
        } else {
            pl.flags.remove(PLAYER_ONGROUND)
        }
    }
}

struct ClipMoveVisitor {
    best: Option<CastResult>,
    vel: na::Vec3<f32>,
    curvel: na::Vec3<f32>,
    pos: na::Vec3<f32>,
    hit_floor: bool,
}
impl PlaneCollisionVisitor for ClipMoveVisitor {
    fn visit_plane(&mut self, plane: &Plane, castresult: &CastResult) {
        let cnorm = if na::dot(&plane.norm, &self.pos) > plane.dist {
            plane.norm * -1.0
        } else {
            plane.norm
        };

        if let Some(CastResult { toi: best_toi, .. }) = self.best {
            if na::approx_eq(&castresult.toi, &best_toi) {
                if clip_velocity(&mut self.curvel, &cnorm) {
                    if cnorm.y < -0.7 {
                        self.hit_floor = true;
                    }
                }
            } else if castresult.toi < best_toi {
                self.curvel = self.vel;
                if clip_velocity(&mut self.curvel, &cnorm) { 
                    self.best = Some(*castresult);
                    if cnorm.y < -0.7 {
                        self.hit_floor = true;
                    }
                }
            }
        } else {
            self.curvel = self.vel;
            if clip_velocity(&mut self.curvel, &cnorm) { 
                self.best = Some(*castresult);
                if cnorm.y < -0.7 {
                    self.hit_floor = true;
                }
            }
        }
    }
    fn should_visit_both(&self) -> bool { true }
}
fn clip_velocity(vel: &mut na::Vec3<f32>, norm: &na::Vec3<f32>) -> bool {
    let d = na::dot(vel, norm);
    if d < 0.0 {
        false
    } else {
        *vel = *vel - (*norm * d);
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use na::{self,
    ApproxEq
    };

    #[test]
    fn movement_clipping() {
        let mut game = ::test::simple_game();
        game.players[0].pos = na::Pnt3::new(0.0, 10.0, 0.0);
        let input = MoveInput {
            wishvel: na::Vec3::new(0.0, -200.0, 0.0)
        };
        move_player(&mut game, 0, &input, 1.0);
        assert_approx_eq!(game.players[0].pos.y, ::player::PLAYER_HALFEXTENTS.y);
        assert_approx_eq!(game.players[0].vel.y, 0.0); 
    }

    #[test]
    fn gravity() {
        let mut game = ::test::simple_game();
        game.movesettings.gravity = 5.0; 
        game.players[0].pos = na::Pnt3::new(0.0, 10.0, 0.0);
        let input = MoveInput {
            wishvel: na::Vec3::new(0.0, 0.0, 0.0)
        };
        move_player(&mut game, 0, &input, 1.0);
        assert_approx_eq!(game.players[0].pos.y, 5.0); 
    }
}

