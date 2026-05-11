use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

use flate2::read::ZlibDecoder;
use tracing::info;

use crate::{
    cache::Cache,
    world::{Chunk, RawChunkNbt, world_manager::ChunkKey},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_new::new)]
pub struct RegionKey(i32, i32);

impl RegionKey {
    pub fn get_file_name(&self) -> String {
        format!("r.{}.{}.mca", self.0, self.1)
    }
}

#[derive(Debug, Clone)]
pub struct Region {
    /// Table d'offsets pré-parsée : 1024 entrées × 4 octets
    /// Format stocké : [offset_24bits << 8 | count_8bits]
    offsets: [u32; 1024],
    path: PathBuf,
}

impl Region {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let mut offsets = [0u32; 1024];
        let mut buf = [0u8; 4];

        for i in 0..1024 {
            file.seek(SeekFrom::Start(i as u64 * 4))?;
            file.read_exact(&mut buf)?;
            offsets[i] = u32::from_be_bytes(buf);
        }
        Ok(Self { offsets, path })
    }

    fn get_offset(&self, local_x: usize, local_z: usize) -> Option<(u32, u8)> {
        let idx = local_z * 32 + local_x;
        let entry = self.offsets[idx];
        if entry == 0 {
            return None;
        }
        Some((entry >> 8, (entry & 0xFF) as u8))
    }

    pub fn load_chunk(&self, lx: usize, lz: usize) -> Result<Option<RawChunkNbt>, String> {
        let (sector, _count) = match self.get_offset(lx, lz) {
            Some(off) => off,
            None => return Ok(None),
        };

        let mut file = File::open(&self.path).map_err(|e| format!("Open region: {e}"))?;

        file.seek(SeekFrom::Start(sector as u64 * 4096))
            .map_err(|e| format!("Seek: {e}"))?;

        // Header chunk : [len_32bits_be][compression_8bits]
        let mut hdr = [0u8; 5];
        file.read_exact(&mut hdr)
            .map_err(|e| format!("Header: {e}"))?;

        let payload_len = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let compression = hdr[4];

        if compression != 2 || payload_len < 10 {
            return Err(format!(
                "Invalid header: comp={compression}, len={payload_len}"
            ));
        }

        // Lecture + décompression Zlib
        let mut compressed = vec![0u8; (payload_len - 1) as usize];
        file.read_exact(&mut compressed)
            .map_err(|e| format!("Read payload: {e}"))?;

        let mut nbt = Vec::with_capacity(payload_len as usize * 3);
        ZlibDecoder::new(&compressed[..])
            .read_to_end(&mut nbt)
            .map_err(|e| format!("Decompress: {e}"))?;

        RawChunkNbt::from_bytes(&nbt)
            .map(Some)
            .map_err(|e| format!("Parse NBT: {e}"))
    }
}

// impl Region {
//    pub fn
// }

#[derive(Debug)]
pub struct ChunkIo {
    base_path: PathBuf,
    regions_cache: Cache<RegionKey, Region>,
}

impl Default for ChunkIo {
    fn default() -> Self {
        Self {
            base_path: PathBuf::default(),
            regions_cache: Cache::new(),
        }
    }
}

impl ChunkIo {
    pub fn new<P: AsRef<Path>>(base: P) -> Result<Self, String> {
        let base_path = base.as_ref().to_path_buf();
        if base_path.is_dir() == false {
            return Err("Is not a directory".to_string());
        }
        Ok(Self {
            base_path,
            ..Default::default()
        })
    }

    /// Load Region from cache or file (if not in cache)
    fn get_region(&self, key: RegionKey) -> Result<Option<Arc<Region>>, String> {
        if let Some(region) = self.regions_cache.get(&key) {
            return Ok(Some(region));
        }

        // Cache miss → load from disk
        let path = self.base_path.join("region").join(key.get_file_name());

        let rx = key.0;
        let rz = key.1;
        let region = match Region::load(&path) {
            Ok(r) => r,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            e => return Err(format!("Load region {rx},{rz}: {:?}", e)),
        };

        let arc = Arc::new(region);
        self.regions_cache
            .insert(key, Region::load(&path).map_err(|e| format!("Load: {e}"))?);
        Ok(Some(arc))
    }

    /// Load Chunk from cache or file (if not in cache)
    pub fn load_chunk(&self, key: ChunkKey) -> Result<Option<Arc<Chunk>>, String> {
        let region_key = key.region_key();
        let region = match self.get_region(region_key)? {
            Some(r) => r,
            None => {
                info!("Region cache miss: {:?}", key);
                return Ok(None);
            }
        };
        info!("Region cache hit: {:?}", key);

        let (lx, lz) = key.region_local();
        let raw = match region.load_chunk(lx, lz)? {
            Some(r) => r,
            None => return Ok(None),
        };

        Chunk::from_raw_nbt(raw)
            .map(|c| Arc::new(c))
            .map(Some)
            .map_err(|e| e.to_string())
    }

    // pub fn load_chunk(&self, key: ChunkKey) -> Result<Option<RawChunkNbt>, String> {
    //     let (rx, rz) = key.region_key();
    //     let region_path = self
    //         .base_path
    //         .join("region")
    //         .join(format!("r.{}.{}.mca", rx, rz));

    //     let mut file = match File::open(&region_path) {
    //         Ok(f) => f,
    //         Err(_) => return Ok(None),
    //     };

    //     let metadata = file.metadata();
    //     if metadata.len() < 8192 {
    //         return Err("File too small, empty/corrupted".to_string());
    //     }

    //     // 1️⃣ Lire l'offset (4 bytes big-endian)
    //     let (cx, cz) = key.region_pad();
    //     let idx = (cz * 32 + cx) as u64;
    //     file.seek(SeekFrom::Start(idx * 4)).ok()?;

    //     let mut offset_buf = [0u8; 4];
    //     file.read_exact(&mut offset_buf)
    //         .inspect_err(|e| eprintln!("❌ Failed to read offset: {}", e))
    //         .ok()?;

    //     eprintln!(
    //         "🔢 Offset table entry: [{:02X} {:02X} {:02X} {:02X}]",
    //         offset_buf[0], offset_buf[1], offset_buf[2], offset_buf[3]
    //     );

    //     // Format: [offset_24bits_be][sector_count_8bits]
    //     let sector_offset =
    //         ((offset_buf[0] as u32) << 16) | ((offset_buf[1] as u32) << 8) | (offset_buf[2] as u32);
    //     let sector_count = offset_buf[3];

    //     if sector_offset == 0 {
    //         eprintln!("⚠️ Chunk not generated in this region file (offset=0)");
    //         return None;
    //     }

    //     eprintln!(
    //         "📍 Sector offset: {}, count: {} (data starts at byte {})",
    //         sector_offset,
    //         sector_count,
    //         sector_offset * 4096
    //     );

    //     // 2️⃣ Positionnement au chunk
    //     file.seek(SeekFrom::Start(sector_offset as u64 * 4096))
    //         .ok()?;

    //     let mut debug_buf = [0u8; 16];
    //     file.read_exact(&mut debug_buf).ok()?;
    //     eprintln!(
    //         "🔍 Raw bytes at offset {}: {:02X?}",
    //         sector_offset * 4096,
    //         debug_buf
    //     );
    //     file.seek(SeekFrom::Start(sector_offset as u64 * 4096))
    //         .ok()?;

    //     // 3️⃣ Header du chunk: [length_24bits_be][compression_type]
    //     let mut header = [0u8; 5]; // 4 bytes length + 1 byte compression
    //     file.read_exact(&mut header).ok()?;

    //     let payload_len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    //     let compression = header[4];

    //     eprintln!(
    //         "📦 Chunk header: len={}, compression={}",
    //         payload_len, compression
    //     );

    //     if compression != 2 {
    //         eprintln!("❌ Expected Zlib (2), got {}", compression);
    //         return None;
    //     }
    //     if payload_len < 1 || payload_len > sector_count as u32 * 4096 {
    //         eprintln!("❌ Invalid payload length");
    //         return None;
    //     }

    //     // 4️⃣ Lecture et décompression
    //     let mut compressed = vec![0u8; (payload_len - 1) as usize];
    //     file.read_exact(&mut compressed).ok()?;

    //     let mut nbt_bytes = Vec::with_capacity(payload_len as usize * 3);
    //     ZlibDecoder::new(&compressed[..])
    //         .read_to_end(&mut nbt_bytes)
    //         .inspect_err(|e| eprintln!("❌ Decompression failed: {}", e))
    //         .ok()?;

    //     eprintln!("✅ Decompressed {} bytes of NBT", nbt_bytes.len());
    //     if nbt_bytes.len() >= 4 {
    //         eprintln!(
    //             "🔍 NBT header: {:02X} {:02X} {:02X} {:02X}...",
    //             nbt_bytes[0], nbt_bytes[1], nbt_bytes[2], nbt_bytes[3]
    //         );
    //     }

    //     // 5️⃣ Parsing NBT
    //     let raw = RawChunkNbt::from_bytes(&nbt_bytes)
    //         .inspect_err(|e| eprintln!("❌ NBT parsing failed: {}", e))
    //         .ok()?;

    //     Ok(Some(raw))
    // }
}

pub fn list_existing_chunks(
    region_path: impl AsRef<Path>,
) -> Result<Vec<(usize, usize)>, std::io::Error> {
    let mut file = File::open(region_path)?;
    let mut existing = Vec::new();

    for entry in 0..1024 {
        // Lire l'offset de chaque entrée de la table
        file.seek(SeekFrom::Start(entry as u64 * 4))?;
        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)?;

        // Les 3 premiers octets = offset en secteurs (big-endian)
        let sector = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);

        if sector != 0 {
            // Convertir l'index linéaire en coords locales (0..32)
            let local_x = entry % 32;
            let local_z = entry / 32;
            existing.push((local_x, local_z));
        }
    }
    Ok(existing)
}

// ─────────────────────────────────────────────────────────────
// Helper pour convertir coords locales → coords monde
// ─────────────────────────────────────────────────────────────
pub fn local_to_world(region_x: i32, region_z: i32, local_x: usize, local_z: usize) -> (i32, i32) {
    (
        region_x * 32 + local_x as i32,
        region_z * 32 + local_z as i32,
    )
}
