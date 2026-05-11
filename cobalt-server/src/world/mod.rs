use bytemuck::cast_slice;
use fastnbt::{ByteArray, error::Result as NbtResult, from_bytes};
use serde::Deserialize;
use tracing::info;

use crate::world::world_manager::ChunkKey;

pub mod chunk_generator;
pub mod chunk_io;
pub mod world_manager;

#[derive(Debug, Deserialize)]
pub struct RawChunkNbt {
    #[serde(rename = "Level")]
    pub data: RawChunkData,
}

impl RawChunkNbt {
    pub fn from_bytes(bytes: &[u8]) -> NbtResult<Self> {
        from_bytes(bytes)
    }
}

#[derive(Debug, Deserialize)]
pub struct RawChunkData {
    #[serde(rename = "xPos")]
    pub x: i32,
    #[serde(rename = "zPos")]
    pub z: i32,
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    #[serde(rename = "TerrainPopulated", default)]
    pub terrain_populated: bool,

    #[serde(rename = "Biomes")]
    pub biomes: ByteArray,

    #[serde(rename = "Sections")]
    pub sections: Vec<RawSection>,
}

#[derive(Debug, Deserialize)]
pub struct RawSection {
    #[serde(rename = "Y")]
    pub y: i8,
    #[serde(rename = "Blocks")]
    pub blocks: ByteArray,
    #[serde(rename = "Data")]
    pub data: ByteArray,
    #[serde(rename = "SkyLight")]
    pub sky_light: ByteArray,
    #[serde(rename = "BlockLight")]
    pub block_light: ByteArray,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub sections: [Section; 16],
    pub biomes: [u8; 256],
    pub terrain_populated: bool,
}

impl Chunk {
    pub fn generate_empty(key: ChunkKey) -> Self {
        Chunk {
            x: key.x(),
            z: key.z(),
            sections: [Section::default(); 16],
            biomes: [0; 256],
            terrain_populated: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Section {
    pub y: i8,
    pub blocks: [u8; 4096],
    pub data: [u8; 2048],
    pub sky_light: [u8; 2048],
    pub block_light: [u8; 2048],
}

impl Default for Section {
    fn default() -> Self {
        Self {
            y: 0,
            blocks: [0; 4096],
            data: [0; 2048],
            sky_light: [0; 2048],
            block_light: [0; 2048],
        }
    }
}

impl Section {
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|&b| b == 0)
    }
}

impl Chunk {
    /// Sérialise un chunk au format binaire 1.8.9
    /// Retourne (bitmask, payload_bytes)
    pub fn serialize_payload(&self, is_overworld: bool, full_chunk: bool) -> (u16, Vec<u8>) {
        // 1️⃣ Calcul du bitmask
        let mut bitmask = 0u16;
        for (i, sec) in self.sections.iter().enumerate() {
            if !sec.is_empty() {
                bitmask |= 1 << i;
            }
        }
        if full_chunk && bitmask == 0 {
            bitmask = 1;
        }

        let sections_count = bitmask.count_ones() as usize;

        // 2️⃣ Pré-allocation séparée par type de données
        let mut all_blocks = Vec::with_capacity(sections_count * 8192); // 4096 blocks × 2 bytes
        let mut all_block_light = Vec::with_capacity(sections_count * 2048); // 2048 bytes
        let mut all_sky_light = Vec::with_capacity(sections_count * 2048); // 2048 bytes

        // 3️⃣ PASS 1 : Collecter TOUS les blocks combinés (id << 4 | meta) en LE
        for (i, sec) in self.sections.iter().enumerate() {
            if (bitmask & (1 << i)) == 0 {
                continue;
            }

            for idx in 0..4096 {
                let block_id = sec.blocks[idx] as u16;
                let meta_byte = sec.data[idx / 2];
                let metadata = if idx % 2 == 0 {
                    (meta_byte >> 4) & 0xF
                } else {
                    meta_byte & 0xF
                };
                let combined = (block_id << 4) | (metadata as u16);
                all_blocks.extend_from_slice(&combined.to_le_bytes());
            }
        }

        // 4️⃣ PASS 2 : Collecter TOUS les block_light
        for (i, sec) in self.sections.iter().enumerate() {
            if (bitmask & (1 << i)) == 0 {
                continue;
            }
            all_block_light.extend_from_slice(&sec.block_light);
        }

        // 5️⃣ PASS 3 : Collecter TOUS les sky_light (seulement Overworld)
        if is_overworld {
            for (i, sec) in self.sections.iter().enumerate() {
                if (bitmask & (1 << i)) == 0 {
                    continue;
                }
                all_sky_light.extend_from_slice(&sec.sky_light);
            }
        }

        // 6️⃣ Assemblage final dans l'ordre PROTOCOLE 1.8.9
        let mut data = Vec::with_capacity(
            all_blocks.len()
                + all_block_light.len()
                + all_sky_light.len()
                + if full_chunk { 256 } else { 0 },
        );

        data.extend(all_blocks); // 🔥 Tous les blocks d'abord
        data.extend(all_block_light); // 🔥 Puis tous les block_light
        if is_overworld {
            data.extend(all_sky_light); // 🔥 Puis tous les sky_light
        }
        if full_chunk {
            data.extend_from_slice(&self.biomes); // 🔥 Enfin les biomes
        }

        (bitmask, data)
    }
    /// Helper pour vérifier si une section est vide (optimisation)
    #[inline]
    fn is_empty(&self) -> bool {
        self.sections
            .iter()
            .all(|s| s.blocks.iter().all(|&b| b == 0))
    }

    pub fn get_block(&self, x: u8, y: u8, z: u8) -> (u8, u8) {
        debug_assert!(x < 16 && z < 16, "Coords hors limites");

        let section_idx = (y / 16) as usize;
        let local_y = y % 16;

        let block_idx = ((local_y as usize) << 8) | ((z as usize) << 4) | (x as usize);

        let section = &self.sections[section_idx];
        let block_id = section.blocks[block_idx];

        let meta_byte = section.data[block_idx / 2];
        let metadata = if block_idx % 2 == 0 {
            (meta_byte >> 4) & 0xF
        } else {
            meta_byte & 0xF
        };

        (block_id, metadata)
    }

    pub fn set_block(&mut self, x: u8, y: u8, z: u8, block_id: u8, metadata: u8) {
        debug_assert!(x < 16 && z < 16, "Coords hors limites");
        debug_assert!(metadata < 16, "Metadata doit être 0-15");

        let section_idx = (y / 16) as usize;
        let local_y = y % 16;
        let block_idx = ((local_y as usize) << 8) | ((z as usize) << 4) | (x as usize);

        let section = &mut self.sections[section_idx];

        section.blocks[block_idx] = block_id;
        let meta_idx = block_idx / 2;
        info!("Set block {} {} {}", x, y, block_id);
        if block_idx % 2 == 0 {
            section.data[meta_idx] = (section.data[meta_idx] & 0x0F) | (metadata << 4);
        } else {
            section.data[meta_idx] = (section.data[meta_idx] & 0xF0) | metadata;
        }
    }

    pub fn from_raw_nbt(raw: RawChunkNbt) -> Result<Self, String> {
        let data = raw.data;

        let biomes = cast_byte_array::<256>(data.biomes.into_inner())?;

        let mut sections = [Section::default(); 16];
        for raw_sec in data.sections {
            let y = raw_sec.y as usize;
            if y >= 16 {
                continue;
            }

            let blocks = cast_byte_array::<4096>(raw_sec.blocks.into_inner())?;
            let data_arr = cast_byte_array::<2048>(raw_sec.data.into_inner())?;
            let sky = cast_byte_array::<2048>(raw_sec.sky_light.into_inner())?;
            let block = cast_byte_array::<2048>(raw_sec.block_light.into_inner())?;

            if blocks.len() != 4096 || data_arr.len() != 2048 {
                continue;
            }

            let sec = &mut sections[y];
            sec.y = raw_sec.y;
            sec.blocks.copy_from_slice(&blocks);
            sec.data.copy_from_slice(&data_arr);
            sec.sky_light.copy_from_slice(&sky);
            sec.block_light.copy_from_slice(&block);
        }

        Ok(Chunk {
            x: data.x,
            z: data.z,
            sections,
            biomes,
            terrain_populated: data.terrain_populated,
        })
    }
}

fn cast_byte_array<const N: usize>(vec: Vec<i8>) -> Result<[u8; N], String> {
    if vec.len() != N {
        return Err(format!("Got {}, expected {}", vec.len(), N));
    }
    let slice: &[u8] = cast_slice(vec.as_slice());
    Ok(slice.try_into().unwrap())
}
