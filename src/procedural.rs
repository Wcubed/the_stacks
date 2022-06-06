use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;

pub struct ProceduralPlugin;

impl Plugin for ProceduralPlugin {
    fn build(&self, app: &mut App) {
        // TODO (Wybe 2022-06-05): generate a seed.
        app.insert_resource(SeededHasherResource::new(42));
    }
}

/// Resource providing pre-seeded hashers for procedural generation.
/// TODO (Wybe 2022-06-05): Is the default hasher good for semi-random generation?
pub struct SeededHasherResource(DefaultHasher);

impl SeededHasherResource {
    pub fn new(seed: u32) -> Self {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        SeededHasherResource(hasher)
    }

    pub fn with<T>(&self, value: T) -> HashedRng
    where
        T: Hash,
    {
        let mut hasher = self.0.clone();
        value.hash(&mut hasher);

        HashedRng(hasher)
    }
}

/// A pseudo-random number generator that is based purely on hashing.
/// It is pre-seeded by getting one from the [SeededHasherResource].
/// This means that each time a new HashedRng is created, and provided with the same arguments,
/// it will provide the same "random" values.
/// This allows for repeatability in procedural generation, that does not depend on which
/// module of the code gets called before, and which one after, because each module will
/// have its own pre-seeded `HashedRng`.
///
/// ```
/// # use crate::the_stacks::procedural::SeededHasherResource;
///
/// // This resource can be added to a bevy `World`.
/// let hasher_resource = SeededHasherResource::new(42);
///
/// // Later in a `System` we can use the resource.
/// // Here we provide a static value, but one could for example provide an entity's id.
/// let mut hasher = hasher_resource.with(0);
/// // Each call to `next_value` will provide a different value.
/// let value1 = hasher.value();
/// let value2 = hasher.value();
///
/// assert_ne!(value1, value2);
/// ```
pub struct HashedRng(DefaultHasher);

impl HashedRng {
    pub fn with<T>(&mut self, value: T)
    where
        T: Hash,
    {
        value.hash(&mut self.0);
    }

    pub fn value(&mut self) -> u64 {
        let value = self.0.finish();
        // Advance the hasher, so the next call to this function will have a completely different value.
        // TODO (Wybe 2022-06-05): is this an appropriate way to do hashed "random" number generation?
        1.hash(&mut self.0);

        value
    }

    pub fn value_in_range(&mut self, range: Range<usize>) -> usize {
        let range_size = range.end - range.start;
        range.start + self.value() as usize % range_size
    }
}
