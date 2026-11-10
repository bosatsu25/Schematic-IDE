pub fn bits_per_block(palette_len: usize) -> u32 {
    let mut b = 2;
    while (1usize << b) < palette_len {
        b += 1;
    }
    b
}

pub fn unpack_block_states(longs: &[i64], total_blocks: usize, bits: u32) -> Vec<usize> {
    if total_blocks == 0 || bits == 0 {
        return Vec::new();
    }
    let mask = (1u64 << bits) - 1;
    let mut indices = Vec::with_capacity(total_blocks);

    for k in 0..total_blocks {
        let start_bit = k * bits as usize;
        let start_long = start_bit / 64;
        let bit_offset = start_bit % 64;

        if start_long >= longs.len() {
            indices.push(0);
            continue;
        }

        let val = if bit_offset + bits as usize <= 64 {
            ((longs[start_long] as u64) >> bit_offset) & mask
        } else {
            let part1 = (longs[start_long] as u64) >> bit_offset;
            let bits1 = 64 - bit_offset;
            let bits2 = bits as usize - bits1;
            let mask2 = (1u64 << bits2) - 1;
            let part2 = if start_long + 1 < longs.len() {
                (longs[start_long + 1] as u64) & mask2
            } else {
                0
            };
            part1 | (part2 << bits1)
        };

        indices.push(val as usize);
    }

    indices
}

/// Reads entry `k` of a tightly packed array. Entries past the end read as 0.
#[inline]
pub fn read_packed(longs: &[i64], k: u64, bits: u32) -> usize {
    let start_bit = k * bits as u64;
    let start_long = (start_bit / 64) as usize;
    let bit_offset = (start_bit % 64) as u32;
    let Some(&first) = longs.get(start_long) else {
        return 0;
    };
    let mask = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let mut value = (first as u64) >> bit_offset;
    if bit_offset + bits > 64 {
        if let Some(&next) = longs.get(start_long + 1) {
            value |= (next as u64) << (64 - bit_offset);
        }
    }
    (value & mask) as usize
}

/// Number of longs a tightly packed array of `count` entries needs.
pub fn packed_len(count: u64, bits: u32) -> u64 {
    (count as u128 * bits as u128).div_ceil(64) as u64
}

/// Streaming writer for a tightly packed array (no intermediate index list).
pub struct BitPacker {
    longs: Vec<i64>,
    bits: u32,
    next: u64,
}

impl BitPacker {
    pub fn new(count: u64, bits: u32) -> Self {
        Self {
            longs: vec![0i64; packed_len(count, bits) as usize],
            bits,
            next: 0,
        }
    }

    /// Appends one entry; entries beyond the declared count are ignored.
    #[inline]
    pub fn push(&mut self, value: usize) {
        let mask = if self.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        };
        let value = value as u64 & mask;
        let start_bit = self.next * self.bits as u64;
        self.next += 1;
        let start_long = (start_bit / 64) as usize;
        let bit_offset = (start_bit % 64) as u32;
        let Some(first) = self.longs.get_mut(start_long) else {
            return;
        };
        *first = ((*first as u64) | (value << bit_offset)) as i64;
        if bit_offset + self.bits > 64 {
            if let Some(next) = self.longs.get_mut(start_long + 1) {
                *next = ((*next as u64) | (value >> (64 - bit_offset))) as i64;
            }
        }
    }

    pub fn finish(self) -> Vec<i64> {
        self.longs
    }
}

pub fn pack_block_states(indices: &[usize], bits: u32) -> Vec<i64> {
    if indices.is_empty() || bits == 0 {
        return Vec::new();
    }
    let total_bits = indices.len() * bits as usize;
    let total_longs = total_bits.div_ceil(64);
    let mut longs = vec![0i64; total_longs];
    let mask = (1u64 << bits) - 1;

    for (k, &idx) in indices.iter().enumerate() {
        let value = (idx as u64) & mask;
        let start_bit = k * bits as usize;
        let start_long = start_bit / 64;
        let bit_offset = start_bit % 64;

        if bit_offset + bits as usize <= 64 {
            let mut u = longs[start_long] as u64;
            u |= value << bit_offset;
            longs[start_long] = u as i64;
        } else {
            let bits1 = 64 - bit_offset;
            let part1 = value & ((1u64 << bits1) - 1);
            let part2 = value >> bits1;

            let mut u1 = longs[start_long] as u64;
            u1 |= part1 << bit_offset;
            longs[start_long] = u1 as i64;

            let mut u2 = longs[start_long + 1] as u64;
            u2 |= part2;
            longs[start_long + 1] = u2 as i64;
        }
    }

    longs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits_per_block() {
        assert_eq!(bits_per_block(1), 2);
        assert_eq!(bits_per_block(2), 2);
        assert_eq!(bits_per_block(3), 2);
        assert_eq!(bits_per_block(4), 2);
        assert_eq!(bits_per_block(5), 3);
        assert_eq!(bits_per_block(8), 3);
        assert_eq!(bits_per_block(9), 4);
        assert_eq!(bits_per_block(16), 4);
        assert_eq!(bits_per_block(17), 5);
    }

    #[test]
    fn streaming_packer_and_reader_match_the_batch_functions() {
        for bits in 2..=17 {
            let max_val = (1usize << bits) - 1;
            let indices: Vec<usize> = (0..333).map(|i| (i * 7919 + 5) % (max_val + 1)).collect();
            let mut packer = BitPacker::new(indices.len() as u64, bits);
            for &i in &indices {
                packer.push(i);
            }
            let longs = packer.finish();
            assert_eq!(longs, pack_block_states(&indices, bits), "bits={bits}");
            assert_eq!(longs.len() as u64, packed_len(indices.len() as u64, bits));
            for (k, &i) in indices.iter().enumerate() {
                assert_eq!(read_packed(&longs, k as u64, bits), i, "bits={bits} k={k}");
            }
            assert_eq!(read_packed(&longs, 1 << 40, bits), 0, "past the end");
        }
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        for bits in 2..=12 {
            let max_val = (1usize << bits) - 1;
            let count = 200;
            let indices: Vec<usize> = (0..count).map(|i| (i * 37 + 13) % (max_val + 1)).collect();

            let packed = pack_block_states(&indices, bits);
            let unpacked = unpack_block_states(&packed, count, bits);
            assert_eq!(unpacked, indices, "Mismatch for bits={bits}");
        }
    }
}
