pub struct ChunkBuilder {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub primary_bitmask: u16,
    pub include_sky_light: bool,
}

impl ChunkBuilder {
    pub fn build_flat_world(&self) -> Vec<u8> {
        let sections_count = self.primary_bitmask.count_ones() as usize;

        // 1.8.9 Data Structure:
        // - Block IDs + Meta (2 bytes per block) * sections
        // - Block Light (0.5 byte per block) * sections
        // - Sky Light (0.5 byte per block) * sections (si overworld)
        // - Biomes (256 bytes)

        let mut all_blocks = Vec::with_capacity(sections_count * 8192);
        let mut all_block_light = Vec::with_capacity(sections_count * 2048);
        let mut all_sky_light = Vec::with_capacity(sections_count * 2048);

        for section_idx in 0..16 {
            // On ne génère que si le bit est présent dans le mask
            if (self.primary_bitmask & (1 << section_idx)) == 0 {
                continue;
            }

            // --- 1. BLOCKS (8192 bytes per section) ---
            for y in 0..16 {
                let global_y = section_idx * 16 + y;
                for z in 0..16 {
                    for x in 0..16 {
                        let id: u16 = match global_y {
                            0 => 7,     // Bedrock
                            1..=4 => 1, // Stone
                            5 => 3,     // Dirt
                            6 => 2,     // Grass
                            _ => 0,     // Air
                        };
                        // Format 1.8: ID est sur les 12 bits de poids fort, meta sur les 4 faibles
                        let block_data = id << 4 | 0;
                        // Little Endian pour les blocs en 1.8
                        all_blocks.extend_from_slice(&block_data.to_le_bytes());
                    }
                }
            }

            // --- 2. BLOCK LIGHT (2048 bytes) ---
            all_block_light.extend_from_slice(&[0x00u8; 2048]); // Noir

            // --- 3. SKY LIGHT (2048 bytes) ---
            all_sky_light.extend_from_slice(&[0xFFu8; 2048]); // Plein jour (0xFF = 15/15 pour 2 blocs)
        }

        // Assemblage final dans l'ordre strict de la 1.8
        let mut raw = Vec::new();
        raw.extend(all_blocks);
        raw.extend(all_block_light);
        if self.include_sky_light {
            raw.extend(all_sky_light);
        }

        // 4. BIOMES (256 bytes) - Seulement si c'est un "Ground-Up Continuous" chunk
        raw.extend_from_slice(&[1u8; 256]); // Plains

        raw
    }
}
