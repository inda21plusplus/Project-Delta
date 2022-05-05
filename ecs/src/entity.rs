use std::collections::HashSet;

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
    generations: Vec<Generation>,
    unused_ids: Vec<EntityId>,
}

impl Entities {
    /// Creates a new `entity`
    /// # Time complexity
    /// *O*(1) (ammortized).
    /// The current implementation keeps a `Vec` of all entities' *generations* which might have to
    /// grow.
    pub fn spawn(&mut self) -> Entity {
        if let Some(id) = self.unused_ids.pop() {
            let gen = self.generations[id as usize];
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
        self.id(entity)
            .map(|id| self.despawn_unchecked(id))
            .is_some()
    }

    /// Despawns the entity with id `id`. Does not check generation or if `id` is already currently
    /// despawned.
    pub fn despawn_unchecked(&mut self, id: EntityId) {
        let gen = &mut self.generations[id as usize];
        if *gen == Generation::MAX {
            // TODO: we're not doomed in this scenario. We can still mark this id as no longer
            // usable somehow.
            panic!("Generation counter for entity id {} has overflown.", id);
        }
        *gen += 1;
        self.unused_ids.push(id);
    }

    /// Indicates whether `entity` still is alive.
    /// # Time complexity
    /// *O*(1)
    pub fn exists(&self, entity: Entity) -> bool {
        let Entity { id, gen } = entity;
        self.generations[id as usize] == gen
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

    fn create_new_id(&mut self) -> EntityId {
        let id = self
            .generations
            .len()
            .try_into()
            // The bookkeeping alone for all those entities would require more than 17 GB so this
            // shouldn'n be an issue.
            .expect("Max entity count (4 294 967 296) exceeded");
        self.generations.push(0);
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
            curr: 0,
            entities,
            unused_ids: entities.unused_ids.iter().copied().collect(),
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
        while (self.curr as usize) < self.entities.generations.len() {
            let curr = self.curr;
            self.curr += 1;
            if self.unused_ids.contains(&curr) {
                continue;
            } else {
                let gen = self.entities.generations[curr as usize];
                return Some(Entity { id: curr, gen });
            }
        }
        None
    }
}
