// EXPLOSION !!!!
// литературная ценность нулевая
// как не надо писать документацию

use crate::actors::{
    Actor, Addr,
    send, actor,
    Sys, ActorContext,
};

pub struct KonoSuba {}

impl ActorContext for KonoSuba {}

impl KonoSuba {
    fn new() -> Self {
        Self {}
    }
    fn within<F, A>(&self, _at: [f32; 2], _r: f32, _visitor: F)
        where
            F: FnMut(&mut Addr<A>),
            A: Actor<Context=Self>,
    {
        // bla bla bla
    }
}

actor! {
    pub struct Megumin<KonoSuba> {
        explosion_range: f32,
        explosion_power: f32,
        mana: isize,
    }

    impl Megumin {
        fn name_en(&mut self, _ctx, _msg: {}) -> String {
            "Megumin".to_string()
        }
        fn name_jp(&mut self, _ctx, _msg: {}) -> String {
            "めぐみん".to_string()
        }
        fn explosion(&mut self, ctx, msg: { at: [f32; 2] }) -> () {
            println!("EXPLOSION!!!");
            self.mana = 0;
            ctx.within::<_, CanBeDestroyed>(msg.at, self.explosion_range, |thing| {
                send![ thing.damage(power: self.explosion_power) ];
            });
        }
    }
}

actor! {
    struct SommelierExplosions<KonoSuba> {
    }

    impl SommelierExplosions {
        /*
        fn Megumin::explosion(&mut self, ctx, msg: { at: [f32; 2] }) -> () {
        }
        */

        fn booom(&mut self, ctx, msg: { volume: f32 }) -> () {
            // TODO
            let _ = ();
        }
    }
}

actor! {
    struct CanBeDestroyed<KonoSuba> {
        hp: f32,
    }

    impl CanBeDestroyed {
        fn damage(&mut self, ctx, msg: { power: f32 }) -> () {
            self.hp -= msg.power;
            if self.hp <= 0.0 {
                println!("destroyed");
            }
        }
    }
}

#[test]
fn one_day() {
    let mut sys = Sys::new(KonoSuba::new());
    let megumin = sys.spawn(Megumin {
        explosion_range: 100.0,
        explosion_power: 9999.0,
        mana: 100_500,
    });

    send![ megumin.explosion(at: [0.0, 0.0]) ];
}
