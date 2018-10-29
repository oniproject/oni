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
    try_trait,
    int_to_from_bytes,
)]

#[macro_use] extern crate log;
#[macro_use] extern crate specs_derive;
#[macro_use] extern crate shred_derive;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate lazy_static;
//#[macro_use] extern crate either;

macro_rules! decelerator {
    () => {
        if true {
            std::thread::sleep(std::time::Duration::from_micros(5));
            oni_trace::scope![decelerator];
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
    }
}

mod bit_io;
//mod morton;

mod ai;
mod prot;

mod components;

mod input;
mod client;
mod server;
mod util;

mod ui;

pub use kiss2d::clrs;

/*
mod clrs {
    #![allow(dead_code)]
    #![allow(clippy::unreadable_literal)]

    use crate::util::color;

    // from http://clrs.cc/
    pub const NAVY: [f32; 3]    = color(0x001F3F);
    pub const BLUE: [f32; 3]    = color(0x0074D9);
    pub const AQUA: [f32; 3]    = color(0x7FDBFF);
    pub const TEAL: [f32; 3]    = color(0x39CCCC);
    pub const OLIVE: [f32; 3]   = color(0x3D9970);
    pub const GREEN: [f32; 3]   = color(0x2ECC40);
    pub const LIME: [f32; 3]    = color(0x01FF70);
    pub const YELLOW: [f32; 3]  = color(0xFFDC00);
    pub const ORANGE: [f32; 3]  = color(0xFF851B);
    pub const RED: [f32; 3]     = color(0xFF4136);
    pub const MAROON: [f32; 3]  = color(0x85144B);
    pub const FUCHSIA: [f32; 3] = color(0xF012BE);
    pub const PURPLE: [f32; 3]  = color(0xB10DC9);
    pub const BLACK: [f32; 3]   = color(0x111111);
    pub const GRAY: [f32; 3]    = color(0xAAAAAA);
    pub const SILVER: [f32; 3]  = color(0xDDDDDD);
    pub const WHITE: [f32; 3]   = color(0xFFFFFF);
}
*/

mod consts {
    #![allow(dead_code)]
    #![allow(clippy::unreadable_literal)]

    use std::time::Duration;

    pub use kiss2d::clrs::*;

    pub const AREA_X: (f32, f32) = (-14.0, 14.0);
    pub const AREA_Y: (f32, f32) = (-4.0, 4.0);

    pub const AREA_W: f32 = AREA_X.1 - AREA_X.0;
    pub const AREA_H: f32 = AREA_Y.1 - AREA_Y.0;


    pub const UPDATE_RATE: f32 = 30.0;
    pub const FRAME_TIME: f32 = 1.0 / UPDATE_RATE;

    pub const SERVER_UPDATE_RATE: f32 = 30.0;
    pub const CLIENT_UPDATE_RATE: f32 = UPDATE_RATE;

    pub const RENDER_TIME: Duration = Duration::from_millis(100);
        //crate::util::secs_to_duration(1.0 / SERVER_UPDATE_RATE);

    pub const SIMULATOR_CONFIG: oni::SimulatorConfig = oni::SimulatorConfig {
        latency: Duration::from_millis(150),
        jitter: Duration::from_millis(0),
        loss: 0.0,
    };

    pub const BOT_COUNT: usize = 100;

    pub const PROTOCOL_ID: u64 =  0x1122334455667788;
    pub const CONNECT_TOKEN_EXPIRY: u32 = 30;
    pub const CONNECT_TOKEN_TIMEOUT: u32 = 5;

    lazy_static! {
        pub static ref PRIVATE_KEY: [u8; 32] = oni::crypto::keygen();
    }

    //pub const FONT_SIZE: f32 = ACTOR_RADIUS * 2.0;
    pub const FONT_SIZE: f32 = 16.0;
    pub const VIEW_SCALE: f32 = 60.0;

    pub const DEFAULT_SPEED: f32 = 2.0;
    pub const ACTOR_RADIUS: f32 = 16.0;

    pub const BG: u32      = BLACK;

    pub const ANOTHER: u32 = LIME;
    pub const CURRENT: u32 = AQUA;
    pub const OTHERS: u32  = SILVER;
    pub const SERVER: u32  = MAROON;
    //pub const INFO: u32    = BLACK;
    pub const LAZER: u32   = RED;
    pub const FIRE: u32    = YELLOW;
    pub const GUN: u32     = TEAL;

    pub const FIRE_RADIUS: f32 = 0.2;
    pub const FIRE_LEN: f32 = 5.0;
}

static FIRA_CODE_REGULAR: &[u8] = include_bytes!("../FiraCode-Regular.ttf");

fn main() {
    static LOGGER: oni_trace::Logger = oni_trace::Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    println!("trace enabled is {}", oni_trace::ENABLED);

    oni_trace::register_thread(None, None);

    {
        // sample
        let module: &'static str = module_path!();
        let file: &'static str = file!();
        let line: u32 = line!();

        println!("{} {}:{}", module, file, line);
    }

    use kiss2d::{Canvas, Font, Key, meter::Meter};
    use std::rc::Rc;

    const WIN_W: usize = 1920;
    const WIN_H: usize = 1080;

    let font = Rc::new(Font::from_bytes(FIRA_CODE_REGULAR).unwrap());
    let mut canvas = Canvas::new("TestBeeed", WIN_W, WIN_H).unwrap();
    let mut meter = Meter::new();

    let mut sim = ui::AppState::new(font);
    while canvas.is_open() && !canvas.is_keydown(Key::Escape) {
        meter.render(&mut canvas, WIN_W as isize - 100, 0);
        canvas.redraw().unwrap();
        canvas.fill(crate::consts::BG);
        sim.render(&mut canvas);
    }

    /*
    //win.set_framerate_limit(None);
    win.set_framerate_limit(Some(60));
    win.set_background_color(BG[0], BG[1], BG[2]);
    win.set_point_size(10.0);
    win.set_light(Light::StickToCamera);

    win.render_loop(sim);
    */
}
