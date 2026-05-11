use std::sync::Arc;

use crate::world::{Chunk, world_manager::ChunkKey};

#[derive(Debug)]
pub struct ChunkGenerator {}

impl Default for ChunkGenerator {
    fn default() -> Self {
        Self {}
    }
}

impl ChunkGenerator {
    pub fn generate_chunk(&self, key: ChunkKey) -> Arc<Chunk> {
        Arc::new(Chunk::generate_empty(key))
    }
}
