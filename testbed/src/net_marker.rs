use fnv::FnvHashMap;
use std::ops::Range;

use specs::{
    prelude::*,
    world::EntitiesRes,
    saveload::{Marker, MarkerAllocator},
};

type T = u16;

/// Marker for entities that should be synced over network
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct NetMarker {
    id: T,
    seq: T,
}

impl Component for NetMarker {
    type Storage = DenseVecStorage<Self>;
}

impl Marker for NetMarker {
    type Identifier = T;
    type Allocator = NetNode;

    fn id(&self) -> Self::Identifier {
        self.id
    }

    // Updates sequence id.
    // Entities with too old sequence id get deleted.
    fn update(&mut self, update: Self) {
        assert_eq!(self.id, update.id);
        self.seq = update.seq;
    }
}

pub trait NetNodeBuilder {
    fn from_server(self, id: u16) -> Self;
}

impl<'a> NetNodeBuilder for specs::world::EntityBuilder<'a> {
    fn from_server(self, id: u16) -> Self {
        let mut alloc = self.world.write_resource::<NetNode>();
        let mut storage = self.world.write_storage::<NetMarker>();
        let m = alloc.allocate(self.entity, Some(id));
        storage.insert(self.entity, m).unwrap();
        self
    }
}

impl<'a> NetNodeBuilder for specs::world::LazyBuilder<'a> {
    fn from_server(self, id: u16) -> Self {
        let entity = self.entity;
        self.lazy.exec(move |world| {
            let mut alloc = world.write_resource::<NetNode>();
            let mut storage = world.write_storage::<NetMarker>();
            let m = alloc.allocate(entity, Some(id));
            storage.insert(entity, m).unwrap();
        });
        self
    }
}

/// Each client and server has one
/// Contains id range and `NetMarker -> Entity` mapping
pub struct NetNode {
    pub range: Range<T>,
    pub mapping: FnvHashMap<T, Entity>,
}

impl NetNode {
    pub fn new(range: Range<T>) -> Self {
        Self {
            range,
            mapping: FnvHashMap::default(),
        }
    }
}

impl MarkerAllocator<NetMarker> for NetNode {
    fn allocate(&mut self, entity: Entity, id: Option<T>) -> NetMarker {
        let id = id.unwrap_or_else(|| {
            self.range.next().expect("Id range must be virtually endless")
        });
        let marker = NetMarker {
            id,
            seq: 0,
        };
        self.mapping.insert(id, entity);
        marker
    }

    fn retrieve_entity_internal(&self, id: T) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<NetMarker>) {
        let i = (&*entities, storage).join().map(|(e, m)| (m.id(), e));
        self.mapping.clear();
        self.mapping.extend(i);
    }
}
