use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::info;

use crate::{
    cache::Cache,
    world::{
        Chunk,
        chunk_generator::ChunkGenerator,
        chunk_io::{ChunkIo, RegionKey},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ChunkKey(i64);

impl ChunkKey {
    #[inline]
    pub const fn new(x: i32, z: i32) -> Self {
        Self(((x as i64) << 32) | (z as i64 & 0xFFFFFFFF))
    }

    #[inline]
    pub const fn x(&self) -> i32 {
        (self.0 >> 32) as i32
    }
    #[inline]
    pub const fn z(&self) -> i32 {
        self.0 as i32
    }

    pub fn region_key(&self) -> RegionKey {
        let rx = self.x().div_euclid(32);
        let rz = self.z().div_euclid(32);
        RegionKey::new(rx, rz)
    }

    pub fn region_local(&self) -> (usize, usize) {
        let cx = self.x().rem_euclid(32) as usize;
        let cz = self.z().rem_euclid(32) as usize;
        (cx, cz)
    }
}

impl Hash for ChunkKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug)]
pub struct WorldManager {
    io: ChunkIo,
    generator: ChunkGenerator,
    chunks: Cache<ChunkKey, Chunk>,
}

impl Default for WorldManager {
    fn default() -> Self {
        Self {
            io: ChunkIo::default(),
            generator: ChunkGenerator::default(),
            chunks: Cache::new(),
        }
    }
}

impl WorldManager {
    pub fn new<P: AsRef<Path>>(base: P) -> Result<Self, String> {
        let io = ChunkIo::new(base)?;
        Ok(Self {
            io,
            ..Default::default()
        })
    }

    pub fn get_chunk(&self, key: ChunkKey) -> Result<Arc<Chunk>, String> {
        if let Some(entry) = self.chunks.get(&key) {
            info!("Global cache hit: {:?}", key);
            return Ok(entry);
        }

        info!("Global cache miss: {:?}", key);

        let chunk = match self.io.load_chunk(key)? {
            Some(r) => r,
            None => {
                info!("Chunk cache miss {:?}", key);
                self.generator.generate_chunk(key)
            }
        };
        info!("Chunk cache hit {:?}", key);

        Ok(self.chunks.insert_return(key, chunk))
    }

    pub fn modify_chunk<F>(&self, key: &ChunkKey, f: F) -> bool
    where
        F: FnOnce(&mut Chunk),
    {
        self.chunks.modify(key, f)
    }
}
