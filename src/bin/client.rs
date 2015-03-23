extern crate glium;
extern crate glutin;
extern crate vel0city;
extern crate wavefront_obj;
extern crate "nalgebra" as na;
extern crate clock_ticks;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate image;

use std::sync::Arc;
use std::borrow::ToOwned;
use glium::DisplayBuild;
use glium::Surface;

use vel0city::assets;
use vel0city::graphics::hud;
use na::{
    Diag,
    Rotation,
    ToHomogeneous,
    Inv
};
use std::f32::consts::{
    PI,
};

pub struct Client {
    playermodel: vel0city::graphics::Model,
    input: vel0city::input::Input,
    hudmanager: vel0city::graphics::hud::HudManager,
    hudelements: Vec<hud::Element>
}
impl Client {
    fn new(display: &glium::Display) -> Client {
        let tex = vec![
            vec![(0u8, 0u8, 0u8), (0u8, 255u8, 0u8)],
            vec![(255u8, 0u8, 0u8), (0, 255u8, 127u8)]
        ];
        let tex = glium::Texture2d::new(display, tex);
        let program = glium::Program::from_source(
            &display,
            &assets::load_str_asset("vertex.glsl").unwrap(),
            &assets::load_str_asset("fragment.glsl").unwrap(),
            None
            ).unwrap();

        let s = assets::load_str_asset("player.obj").unwrap();
        let playerobj = &wavefront_obj::obj::parse(s).unwrap().objects[0];

        let playermodel = vel0city::graphics::wavefront::obj_to_model(playerobj,
                                                                      Arc::new(program),
                                                                      tex,
                                                                      display);
        let input = vel0city::input::Input::new();
        let hudmanager = hud::HudManager::new(display);

        let tex = assets::load_bin_asset("textures/arrow.png").unwrap();
        let tex = image::load(::std::old_io::BufReader::new(&tex), image::PNG).unwrap();
        let tex = glium::Texture2d::new(display, tex);

        fn id(context: &hud::Context) -> na::Mat4<f32> {
            let ang = std::f32::consts::PI - (context.player_vel.x.atan2(context.player_vel.z) - context.eyeang.y);
            let scale = na::norm(&na::Vec2::new(context.player_vel.x, context.player_vel.z)) / 1200.0;
            let scalemat = na::Mat4::from_diag(&na::Vec4::new(scale, scale, scale, 1.0));
            let rotmat = na::Rot3::new(na::Vec3::new(0.0, 0.0, ang)).to_homogeneous();
            rotmat * scalemat
        }

        Client {
            playermodel: playermodel,
            input: input,
            hudmanager: hudmanager,
            hudelements: vec![hud::Element {
                transform: na::Iso2::new(na::zero(), na::zero()),
                element_type: hud::ElementType::TransformedBlit {
                    texture: tex,
                    f: id 
                }
            }]
        }
    }
}

#[cfg(not(test))]
fn main() {
    env_logger::init().unwrap();

    let display = glutin::WindowBuilder::new()
        // .with_vsync()
        .with_title("vel0city".to_owned())
        .with_dimensions(800, 600)
        .build_glium()
        .unwrap();
    let mut client = Client::new(&display);
    let (x, y) = display.get_framebuffer_dimensions();
    let mut drawparams: glium::DrawParameters = std::default::Default::default();
    drawparams.depth_test = glium::DepthTest::IfLess;
    drawparams.depth_write = true;

    let proj = na::Persp3::new(x as f32 / y as f32, 90.0, 0.1, 8192.0).to_mat();

    let mut game = vel0city::Game {
        movesettings: std::default::Default::default(),
        players: vec![vel0city::player::Player {
            pos: na::Pnt3::new(0.0, -10.0, 0.),
            eyeheight: 0.0,
            eyeang: na::zero(), 
            halfextents: vel0city::player::PLAYER_HALFEXTENTS,
            vel: na::zero(),
            flags: vel0city::player::PlayerFlags::empty(),
            landtime: 0.0,
            holdjumptime: 0.0,
        }],
        map: vel0city::map::single_plane_map(),
        timescale: 1.0,
        time: 0.0,
    };

    let asset = assets::load_bin_asset("maps/test.bsp").unwrap();
    let mapmodel = vel0city::qbsp_import::import_graphics_model(&asset, &display).unwrap();
    
    let winsize = display.get_window().unwrap().get_outer_size().unwrap();
    //client.input.cursorpos = (winsize.0 as i32 / 2, winsize.1 as i32 / 2);

    let tick = 1.0/128.0;
    let mut lasttime = clock_ticks::precise_time_s();
    let mut accumtime = 0.0;
    let mut smoothtime = 0.0;
    while !display.is_closed() {
        let curtime = clock_ticks::precise_time_s();
        let frametime = curtime - lasttime;
        accumtime += frametime;
        smoothtime = (smoothtime*16.0 + frametime) / 17.0;
        lasttime = curtime;
        debug!("{}FPS", 1.0 / smoothtime);
        
        let win = display.get_window().unwrap();
        for ev in win.poll_events() {
            client.input.handle_event(&win, &ev);
        }

        let ang = game.players[0].eyeang;
        let rot = na::UnitQuat::new(na::Vec3::new(0.0, ang.y, 0.0));
        let rot = rot.append_rotation(
            &na::Vec3::new(PI + ang.x, 0.0, 0.0)
            );

        let l = na::Iso3::new_with_rotmat(na::zero(), rot.to_rot()).inv().unwrap().to_homogeneous();
        let v = na::Iso3::new((game.players[0].pos.to_vec() + na::Vec3 { y: vel0city::player::PLAYER_HALFEXTENTS.y * -0.6, ..na::zero() }) * -1.0, na::zero()).to_homogeneous();
        //l.inv();
        let view = vel0city::graphics::View {
            w2s: proj * l * v,
            drawparams: drawparams, 
        };

        let mi = client.input.make_moveinput(&game.movesettings);

        if accumtime >= tick {
            while accumtime >= tick {
                accumtime -= tick;
                let timescale = game.timescale; // borrow checker hack
                let time = tick as f32 * timescale;
                game.time += time;
                vel0city::player::movement::move_player(&mut game, 0, &mi, time);
            }
            let pv = game.players[0].vel;
            debug!("Player speed: {}", na::norm(&na::Vec2::new(pv.x, pv.z))); 
        }

        let mut target = display.draw();
        target.clear_depth(1.0);
        vel0city::graphics::draw_view(&game,
                                      &view,
                                      &client.playermodel,
                                      &mapmodel,
                                      &mut target);
        let hudcontext = hud::Context {
            eyeang: game.players[0].eyeang,
            player_vel: game.players[0].vel
        };

        client.hudmanager.draw_elements(&mut target, &hudcontext, &client.hudelements);
        target.finish();
    }
        
}
