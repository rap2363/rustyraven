use crate::memory::Segment;

// Address space from 0x0000 --> 0xFFFF, but
// with mirrors from 0x4000 onward.
struct PpuMemory {
    // 0x0000 --> 0x1FFF
    pattern_tables: Segment<0x2000>,
    // 0x2000 --> 0x2FFF (with mirrors up to 0x3EFF)
    name_tables: Segment<0x1000>,
    // 0x3F00 --> 0x3F20 (with mirrors up to 0x4000)
    palletes: Segment<0x0040>,
}

impl PpuMemory {
     pub fn initialize() -> Self {
        Self {
            pattern_tables: Segment::<0x2000>::initialize(),
            name_tables: Segment::<0x1000>::initialize(),
            palletes: Segment::<0x0040>::initialize(),
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        // First memory map modulo 0x4000.
        let address = address % 0x4000;
        if address < 0x2000 {
            // Pattern Tables
            self.pattern_tables.write_byte(address as usize, value);
        } else if address < 0x3F00 {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            let name_table_address = (address - 0x2000) % 0x1000;
            self.name_tables.write_byte(name_table_address as usize, value);
        } else {
            // Pallete Memory
            let pallete_memory_address = (address - 0x3F00) % 0x20;
            self.palletes.write_byte(pallete_memory_address as usize, value);
        }
    }

    pub fn write_bytes(&mut self, address: u16, values: &[u8]) {
        for i in 0..values.len() {
            self.write_byte(address + (i as u16), values[i]);
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        // First memory map modulo 0x4000.
        let address = address % 0x4000;
        if address < 0x2000 {
            // Pattern Tables
            self.pattern_tables.read_byte(address as usize)
        } else if address < 0x3F00 {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            let name_table_address = (address - 0x2000) % 0x1000;
            self.name_tables.read_byte(name_table_address as usize)
        } else {
            // Pallete Memory
            let pallete_memory_address = (address - 0x3F00) % 0x20;
            self.palletes.read_byte(pallete_memory_address as usize)
        }
    }

    // Returns two bytes assuming little endian. So the bytes
    // come back $HHLL even though they're *read* as LLHH.
    //
    // Note this wraps around the entire memory space!
    pub fn read_two_bytes(&self, address: u16) -> u16 {
        u16::from_le_bytes([
            self.read_byte(address), 
            self.read_byte(address.wrapping_add(1)),
        ])
    }
}

struct Index {
    block_index: u8,
    quadrant: u8,
    tile_x: u8,
    tile_y: u8,
    fine_x: u8,
    fine_y: u8,
}

const BLOCK_SIZE: u8 = 32; // Blocks are 32x32 pixels
const HALF_BLOCK_SIZE: u8 = BLOCK_SIZE / 2;
const TILE_SIZE: u8 = 8; // Tiles are 8x8.

// Returns the Index for a specific pixel.
// (0, 0) <= (x,y) < (256, 240)
// Panics if not within bounds.
fn get_index(x: u8, y: u8) -> Index {
    let block_index = 8 * (y / BLOCK_SIZE) + (x / BLOCK_SIZE);
    let tile_x = x / TILE_SIZE;
    let tile_y = y / TILE_SIZE;
    let fine_x = x % TILE_SIZE;
    let fine_y = y % TILE_SIZE;

    // Quadrants are laid out like the following in a single block:
    // |-------|-------|
    // |   0   |   1   |
    // |-------|-------|
    // |   2   |   3   |
    // |-------|-------|
    // It turns out that they are effectively indexed by taking the bit 5 of x and bit 5 of y.
    // Specifically the quadrant is just y.5|x.5
    let quadrant = (y >> 4 | x >> 5) & 0x11;

    Index { block_index, quadrant, tile_x, tile_y, fine_x, fine_y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_on_almost_last_pixel() {
        let Index {block_index, quadrant, tile_x, tile_y, fine_x, fine_y } = get_index(254, 239);
        assert_eq!(block_index, 63);
        assert_eq!(quadrant, 1);
        assert_eq!(tile_x, 31);
        assert_eq!(tile_y, 29);
        assert_eq!(fine_x, 6);
        assert_eq!(fine_y, 7);
    }
}