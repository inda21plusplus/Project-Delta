use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

type EntityId = u32;
type Generation = u32;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Entity {
    id: EntityId,
    gen: Generation,
}

impl Entity {
    /// Retrieves the id of `self` without checking if `self` is still alive. Most callers should
    /// use `Entities::id` instead.
    pub fn get_id_unchecked(self) -> EntityId {
        self.id
    }
}

/// Currently there can be at most `u32::MAX + 1` entities alive at a time and for every one of
/// those 'slots' there can exist at most `u32::MAX + 1` different entities at a time. If any of
/// these are exceeded there will be a panic.
#[derive(Debug, Default)]
pub struct Entities {
    // TODO: optimize
    generations: RefCell<Vec<Cell<Generation>>>,
    unused_ids: RefCell<Vec<EntityId>>,
}

impl Entities {
    /// Creates a new `entity`
    /// # Time complexity
    /// *O*(1) (ammortized).
    /// The current implementation keeps a `Vec` of all entities' *generations* which might have to
    /// grow.
    pub fn spawn(&self) -> Entity {
        if let Some(id) = self.unused_ids.borrow_mut().pop() {
            let gen = self.generations.borrow()[id as usize].get();
            Entity { id, gen }
        } else {
            Entity {
                id: self.create_new_id(),
                gen: 0,
            }
        }
    }

    /// Returns `true` if the `entity` was despawned and `false` if `entity` had been despawned
    /// previously.
    /// # Time complexity
    /// *O*(1) (ammortized).
    /// The current implementation keeps a `Vec` of currently unused id's which might have to grow.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        // NOTE: requires mutable access since otherwise an `Iter` could iterate over the despawned
        // entitiy.
        self.id(entity)
            .map(|id| self.despawn_unchecked(id))
            .is_some()
    }

    /// Despawns the entity with id `id`. Does not check generation or if `id` is already currently
    /// despawned.
    pub fn despawn_unchecked(&mut self, id: EntityId) {
        let gen = &self.generations.borrow()[id as usize];
        if gen.get() == Generation::MAX {
            // TODO: we're not doomed in this scenario. We can still mark this id as no longer
            // usable somehow.
            panic!("Generation counter for entity id {} has overflown.", id);
        }
        gen.set(gen.get() + 1);
        self.unused_ids.borrow_mut().push(id);
    }

    /// Indicates whether `entity` still is alive.
    /// # Time complexity
    /// *O*(1)
    pub fn exists(&self, entity: Entity) -> bool {
        let Entity { id, gen } = entity;
        self.generations.borrow()[id as usize].get() == gen
    }

    /// Returns the id of `entity` if `entity` is still alive.
    /// # Time complexity
    /// *O*(1)
    pub fn id(&self, entity: Entity) -> Option<EntityId> {
        self.exists(entity).then(|| entity.id)
    }

    /// Creates an iterator over all currently alive entities.
    ///
    /// # Time complexity
    /// Creation: *O*(*u*) where *u* is the amount of currently unused entity ID's.
    /// Iteration: *O*(1) for every call to next.
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    /// Creates an iterator over all currently alive entities, yielding all possible pairs of
    /// entities. If `(A, B)` is yielded, then `(B, A)` is not.
    ///
    /// # Time complexity
    /// Creation: *O*(*u*) where *u* is the amount of currently unused entity ID's.
    /// Iteration: *O*(1) for every call to next.
    pub fn iter_combinations(&self) -> IterCombinations {
        IterCombinations::new(self)
    }

    fn create_new_id(&self) -> EntityId {
        let mut g = self.generations.borrow_mut();
        let id = g
            .len()
            .try_into()
            // The bookkeeping alone for all those entities would require more than 17 GB so this
            // shouldn'n be an issue.
            .expect("Max entity count (4 294 967 296) exceeded");
        g.push(Cell::new(0));
        id
    }
}

pub struct Iter<'e> {
    curr: EntityId,
    entities: &'e Entities,
    unused_ids: HashSet<EntityId>,
}

impl<'e> Iter<'e> {
    fn new(entities: &'e Entities) -> Self {
        Self {
            curr: 1, // skip the resource holder
            entities,
            unused_ids: entities.unused_ids.borrow().iter().copied().collect(),
        }
    }

    /// Get the iter's entities.
    pub fn entities(&self) -> &Entities {
        self.entities
    }
}

impl<'e> Iterator for Iter<'e> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        while (self.curr as usize) < self.entities.generations.borrow().len() {
            let curr = self.curr;
            self.curr += 1;
            if self.unused_ids.contains(&curr) {
                continue;
            } else {
                let gen = self.entities.generations.borrow()[curr as usize].get();
                return Some(Entity { id: curr, gen });
            }
        }
        None
    }
}

pub struct IterCombinations<'e> {
    curr_a: EntityId,
    curr_b: EntityId,
    entities: &'e Entities,
    unused_ids: HashSet<EntityId>,
}

impl<'e> IterCombinations<'e> {
    fn new(entities: &'e Entities) -> Self {
        Self {
            curr_a: 1, // skip the resource holder
            curr_b: 2,
            entities,
            unused_ids: entities.unused_ids.borrow().iter().copied().collect(),
        }
    }

    /// Get the iter's entities.
    pub fn entities(&self) -> &Entities {
        self.entities
    }
}

impl<'e> Iterator for IterCombinations<'e> {
    type Item = (Entity, Entity);

    fn next(&mut self) -> Option<Self::Item> {
        let gen = self.entities.generations.borrow();
        while (self.curr_a as usize) < gen.len() {
            let id_a = self.curr_a;
            if self.unused_ids.contains(&id_a) {
                continue;
            }
            while (self.curr_b as usize) < gen.len() {
                let id_b = self.curr_b;
                self.curr_b += 1;
                if self.unused_ids.contains(&id_b) {
                    continue;
                }
                let gen_a = gen[id_a as usize].get();
                let gen_b = gen[id_b as usize].get();
                return Some((
                    Entity {
                        id: id_a,
                        gen: gen_a,
                    },
                    Entity {
                        id: id_b,
                        gen: gen_b,
                    },
                ));
            }
            self.curr_a += 1;
            self.curr_b = self.curr_a + 1;
        }
        None
    }
}
