#![feature(
    const_fn,
    const_let,
    const_int_ops,
    duration_getters,
    type_ascription,
    decl_macro,
    generators,
    generator_trait,
    ptr_offset_from,
)]

#[macro_use] extern crate specs_derive;
#[macro_use] extern crate shred_derive;
#[macro_use] extern crate serde_derive;
//#[macro_use] extern crate either;

use kiss3d::{
    light::Light,
    window::Window,
    text::Font,
};

mod ai;
mod prot;

mod components;

mod app;
mod input;
mod client;
mod server;
mod util;

mod varint;

mod consts {
    use std::time::Duration;

    use crate::util::color;

    pub static SERVER_UPDATE_RATE: f32 = 10.0;
    pub static CLIENT_UPDATE_RATE: f32 = 50.0;

    pub static RENDER_TIME: Duration =
        Duration::from_millis(100);
        //crate::util::secs_to_duration(1.0 / SERVER_UPDATE_RATE);

    pub const DEFAULT_LATENCY: Duration = Duration::from_millis(100);
    //pub const DEFAULT_JITTER: Duration = Duration::from_millis(20);
    pub const DEFAULT_JITTER: Duration = Duration::from_millis(0);

    pub const FONT_SIZE: f32 = ACTOR_RADIUS * 2.0;

    pub const DEFAULT_SPEED: f32 = 2.0;
    pub const ACTOR_RADIUS: f32 = 16.0;

    // from http://clrs.cc/
    pub const BG: [f32; 3]      = color(0x7FDBFF);

    pub const ANOTHER: [f32; 3] = color(0x0074D9);
    pub const CURRENT: [f32; 3] = color(0x001F3F);
    pub const SERVER: [f32; 3]  = color(0x85144B);
    //pub const INFO: [f32; 3]    = color(0x111111);

    pub const LAZER: [f32; 3]   = color(0xFF4136);
    pub const FIRE: [f32; 3]    = color(0xFFDC00);
    pub const GUN: [f32; 3]     = color(0x3D9970);
}

static FIRA_CODE_REGULAR: &[u8] = include_bytes!("../FiraCode-Regular.ttf");

fn main() {
    use crate::consts::*;
    use std::mem::size_of;

    println!("size_of Input: {}", size_of::<crate::prot::Input>());
    println!("size_of WorldState: {}", size_of::<crate::prot::WorldState>());
    println!("size_of EntityState: {}", size_of::<crate::prot::EntityState>());

    let font = Font::from_bytes(FIRA_CODE_REGULAR).unwrap();
    let mut win = Window::new("TestBeeed");

    let mut mouse = win.add_circle(3.0);
    mouse.set_color(LAZER[0], LAZER[1], LAZER[2]);

    let sim = app::AppState::new(font, mouse);

    win.set_background_color(BG[0], BG[1], BG[2]);
    win.set_point_size(10.0);
    win.set_light(Light::StickToCamera);

    win.render_loop(sim);
}
