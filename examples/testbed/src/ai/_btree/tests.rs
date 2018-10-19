use rand::random;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::{Generator, GeneratorState};

use super::forest::{World, Tree};

fn world_behavior(world: Rc<RefCell<World>>) -> impl Generator<Return=Rc<RefCell<World>>> {
    move || {
        if world.borrow().can_shine() {
            while world.borrow_mut().toggle_sun() {
                yield;
            }
        }

        if world.borrow().can_rain() {
            while world.borrow_mut().rain() {
                yield;
            }
        }

        return world;
    }
}

macro action($obj: ident, $fn:ident) {
    while $obj.borrow_mut().$fn() {
        yield;
    }
}

fn tree_behavior(tree: Rc<RefCell<Tree>>) -> impl Generator<Return=Rc<RefCell<Tree>>> {
    move || {
        //sequence!
        loop {
            // photosynthesise
            if !tree.borrow().can_make_energy() {
                break;
            } else {
                action!(tree, make_energy);
            }

            if !tree.borrow().can_grow() {
                break;
            } else {
                action!(tree, grow);
            }

            if !tree.borrow().can_emit_oxygen() {
                break;
            } else {
                action!(tree, emit_oxygen);
            }

            break;
        }

        if tree.borrow().can_gather_sun() {
            action!(tree, gather_sun);
        }

        if tree.borrow().can_gather_water() {
            action!(tree, gather_water);
        }

        return tree;
    }
}

#[test]
fn run() {
    let world = World::new();
    let world = Rc::new(RefCell::new(world));

    let tree = Tree::new(world.clone());
    let tree = Rc::new(RefCell::new(tree));

    let mut tree_generator = tree_behavior(tree.clone());
    let mut world_generator = world_behavior(world.clone());

    loop {
        {
            match unsafe { tree_generator.resume() } {
                GeneratorState::Yielded(_) => (), //println!("tree yield"),
                GeneratorState::Complete(tree) => {
                    tree_generator = tree_behavior(tree);
                },
            }
        }

        {
            match unsafe { world_generator.resume() } {
                GeneratorState::Yielded(_) => (), //println!("world yield"),
                GeneratorState::Complete(world) => {
                    world_generator = world_behavior(world);
                },
            }
        }

        {
            let world = world.borrow();
            let tree = tree.borrow();

            println!("  Tree: {:2?}m | {:?} water | {:?} sun",
                     tree.height, tree.water, tree.sun);

            let nd = if world.sunny { "day" } else { "night" };
            println!("  World:    | {:?} water | {:?} oxygen | {:5}",
                     world.groundwater, world.oxygen,  nd);

            println!();
        }

        use std::thread::sleep;
        use std::time::Duration;

        sleep(Duration::from_millis(500))
    }
}
