use std::mem;

type EntityId = u32;
type Generation = u32;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Entity {
    id: EntityId,
    gen: Generation,
}

#[derive(Debug)]
enum MaybeEntity {
    Alive(Generation),
    Free(Option<EntityId>),
}

#[derive(Debug, Default)]
pub struct Entities {
    gen_counter: Generation,
    entities: Vec<MaybeEntity>,
    free_id: Option<EntityId>,
}

impl Entities {
    pub fn spawn(&mut self) -> Entity {
        let gen = self.gen_counter;
        let id = if let Some(id) = self.free_id {
            match mem::replace(&mut self.entities[id as usize], MaybeEntity::Alive(gen)) {
                MaybeEntity::Alive(_) => unreachable!(),
                MaybeEntity::Free(next_free) => self.free_id = next_free,
            }
            id
        } else {
            let id: EntityId = self.entities.len().try_into().unwrap();
            self.entities.push(MaybeEntity::Alive(gen));
            id
        };
        let entity = Entity { id, gen };
        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.gen_counter += 1;
        match self.entities[entity.id as usize] {
            MaybeEntity::Free(_) => false,
            MaybeEntity::Alive(_) => {
                let free = self.free_id;
                self.free_id = Some(entity.id);
                self.entities[entity.id as usize] = MaybeEntity::Free(free);

                true
            }
        }
    }

    pub fn exists(&self, entity: Entity) -> bool {
        match self.entities[entity.id as usize] {
            MaybeEntity::Alive(gen) => entity.gen == gen,
            MaybeEntity::Free(_) => false,
        }
    }

    pub fn id(&self, entity: Entity) -> Option<EntityId> {
        if self.exists(entity) {
            Some(entity.id)
        } else {
            None
        }
    }

    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }
}

pub struct Iter<'e> {
    curr: EntityId,
    entities: &'e Entities,
}

impl<'e> Iter<'e> {
    fn new(entities: &'e Entities) -> Self {
        Self { curr: 0, entities }
    }
}

impl<'e> Iterator for Iter<'e> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        while (self.curr as usize) < self.entities.entities.len() {
            let curr = self.curr;
            self.curr += 1;
            match self.entities.entities[curr as usize] {
                MaybeEntity::Alive(gen) => return Some(Entity { id: curr, gen }),
                MaybeEntity::Free(_) => continue,
            }
        }
        None
    }
}
