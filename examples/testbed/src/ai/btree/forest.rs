use rand::random;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::{Generator, GeneratorState};

pub struct World {
    pub groundwater: usize,
    pub oxygen: usize,
    pub sunny: bool,
}

impl World {
    pub fn new() -> Self {
        Self {
            groundwater: 0,
            oxygen: 0,
            sunny: false,
        }
    }

    pub fn is_sunny(&self) -> bool { self.sunny }
    pub fn has_wather(&self) -> bool { self.groundwater > 0 }

    pub fn can_rain(&self) -> bool { random() }
    pub fn can_shine(&self) -> bool { random() }

    pub fn give_wather(&mut self) -> usize {
        self.groundwater -= 1;
        1
    }

    pub fn oxygenate(&mut self) {
        self.oxygen += 1;
    }

    pub fn rain(&mut self) -> bool {
        self.groundwater += 1;
        false
    }
    pub fn toggle_sun(&mut self) -> bool {
        self.sunny = !self.sunny;
        false
    }
}

pub struct Tree {
    pub energy: bool,
    pub oxygen: bool,
    pub height: usize,
    pub water: usize,
    pub sun: usize,
    pub world: Rc<RefCell<World>>,
}

impl Tree {
    pub fn new(world: Rc<RefCell<World>>) -> Self {
        Self {
            world,
            height: 1,
            energy: false,
            oxygen: false,
            water: 0,
            sun: 0,
        }
    }

    pub fn has_water(&self) -> bool { self.water > 0 }
    pub fn has_sun(&self) -> bool { self.sun > 0 }

    pub fn can_grow(&self) -> bool { self.energy }
    pub fn can_make_energy(&self) -> bool { self.has_water() && self.has_sun() }
    pub fn can_emit_oxygen(&self) -> bool { self.oxygen }
    pub fn can_gather_sun(&self) -> bool { self.world.borrow().is_sunny() }
    pub fn can_gather_water(&self) -> bool { self.world.borrow().has_wather() }

    pub fn grow(&mut self) -> bool {
        self.energy = false;
        self.height += 1;
        false
    }
    pub fn make_energy(&mut self) -> bool {
        self.sun -= 1;
        self.water -= 1;
        self.energy = true;
        false
    }
    pub fn emit_oxygen(&mut self) -> bool {
        self.oxygen = false;
        self.world.borrow_mut().oxygenate();
        false
    }
    pub fn gather_sun(&mut self) -> bool {
        self.sun += 1;
        false
    }
    pub fn gather_water(&mut self) -> bool {
        let mut world = self.world.borrow_mut();
        self.water += world.give_wather();
        false
    }
}

use super::{
    Behavior as Behavior,
    Success, Failure,
    State,
    Behavior::{
        Select as Selector,
        Sequence,
        Action,
        If,
        While,
        Wait,
        WaitForever,
        AlwaysSucceed,
        WhenAny,
        WhenAll,
    },
};

#[derive(Clone, Copy, Debug)]
enum WorldActions {
    Sun,
    Rain,
}
use self::WorldActions::{Sun, Rain};

macro action($self:ident, $args:ident, $can:ident, $run:ident) {
    if $self.$can() {
        $self.$run();
        (Success, $args.dt)
    } else {
        (Failure, $args.dt)
    }
}

impl World {
    fn behavior() -> Behavior<WorldActions> {
        //Selector(vec![Action(Sun), Action(Rain)])

        While(Box::new(WaitForever), vec![
            Sequence(vec![
                Action(Sun),
                Wait(0.5),
                Action(Rain),
                Wait(0.5),
            ])
        ])
    }

    fn exec(&mut self, delta: f64, state: &mut State<WorldActions, ()>) {
        state.event(delta, &mut |args| {
            //println!("{:?}", *args.action);
            match *args.action {
                Sun  => action!(self, args, can_shine, toggle_sun),
                Rain => action!(self, args, can_rain , rain),
            }
        });
    }
}

#[derive(Clone, Copy, Debug)]
enum TreeActions {
    MakeEnergy,
    Grow,
    EmitOxygen,
    GatherSun,
    GatherWater,
}

use self::TreeActions::{
    MakeEnergy,
    Grow,
    EmitOxygen,
    GatherSun,
    GatherWater,
};

impl Tree {
    fn exec(&mut self, delta: f64, state: &mut State<TreeActions, ()>) {
        state.event(delta, &mut |args| {
            println!("{:?}", *args.action);
            match *args.action {
                MakeEnergy =>   action!(self, args, can_make_energy , make_energy),
                Grow =>         action!(self, args, can_grow        , grow),
                EmitOxygen =>   action!(self, args, can_emit_oxygen , emit_oxygen),
                GatherSun =>    action!(self, args, can_gather_sun  , gather_sun),
                GatherWater =>  action!(self, args, can_gather_water, gather_water),
            }
        });
    }
    fn behavior() -> Behavior<TreeActions> {
        While(Box::new(WaitForever), vec![
            Selector(vec![
                Sequence(vec![
                    Action(MakeEnergy),
                    //Wait(0.25),
                    Action(Grow),
                    Wait(0.25),
                    Action(EmitOxygen),
                    Wait(0.25),
                ]),
                WhenAny(vec![
                        Wait(0.25),
                    Action(GatherWater),
                        Wait(0.25),
                        //Wait(0.25),
                    Action(GatherSun),
                        Wait(0.25),
                        //Wait(0.25),
                ]),
            ])
        ])
    }
}


#[test]
fn run() {
    let world = World::new();
    let world = Rc::new(RefCell::new(world));
    let mut tree = Tree::new(world.clone());

    let mut tree_state = State::new(Tree::behavior());
    let mut world_state = State::new(World::behavior());

    loop {
        {
            tree.exec(1.0, &mut tree_state);
        }

        {
            let mut world = world.borrow_mut();
            world.exec(1.0, &mut world_state);
        }

        {
            let world = world.borrow();
            println!("  Tree: {:2?}m | {:?} water | {:?} sun",
                     tree.height, tree.water, tree.sun);

            let nd = if world.sunny { "day" } else { "night" };
            println!("  World:    | {:?} water | {:?} oxygen | {:5}",
                     world.groundwater, world.oxygen,  nd);

            println!();
        }

        use std::thread::sleep;
        use std::time::Duration;

        sleep(Duration::from_millis(200))
    }
}
