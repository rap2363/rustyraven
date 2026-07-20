use std::iter::Cycle;

use crate::memory::Segment;
use crate::ppu_registers::PpuControl;

enum PpuRegister {
    Control(u8),
    Status(u8),
    Scroll(u8),
    Address(u8),
    Data(u8),
}

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

// Specific operations to execute each PPU cycle. Note that each of these will take *exactly* one cycle.
#[derive(Debug, Clone, Copy)]
enum CycleOperation {
    Idle,
    BackgroundLsbit,
    NameTableAccess,
    UnusedNameTableAccess,
    IgnoredNameTableAccess,
    AttributeTableAccess,
    BackgroundLsb,
    BackgroundMsb,
    IncrementHorizontalV,
    IncrementVerticalV,
    IncrementHorizontalAndVerticalV,
    EqualizeHorizontalVT,
    EqualizeVerticalVT,
    SetVblank,
    ClearVblankAndSpriteZero,
    SpriteLsb,
    SpriteMsb,
}

struct Ppu {
    memory: PpuMemory,
    control: PpuControl,
    loopy_v: u16,
    loopy_t: u16,
    fine_x: u8,
    loopy_w: WriteToggle,
    frame_operations: [[CycleOperation; 262]; 341],
    frame_index: (usize, usize), // row, column
}

enum WriteToggle {
    First,
    Second,
}

impl Ppu {
    fn initialize() -> Self {
        let mut frame_operations = [[CycleOperation::Idle; 262]; 341];
        // Frame diagram: https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
        // Visible lines + Prerender line.
        for row_index in (0..=239).into_iter().chain(261..262) {
            let mut scanline = frame_operations[row_index];
            scanline[0] = CycleOperation::BackgroundLsbit;
            // We do this for 256 pixels in 8-bit increments (so 256 / 8 = 32)
            for x in 0..32 {
                let offset = 8 * x;
                scanline[offset + 1] = CycleOperation::NameTableAccess;
                scanline[offset + 3] = CycleOperation::AttributeTableAccess;
                scanline[offset + 5] = CycleOperation::BackgroundLsb;
                scanline[offset + 7] = CycleOperation::BackgroundMsb;
                scanline[offset + 8] = if x == 31 {
                    CycleOperation::IncrementHorizontalAndVerticalV
                } else {
                    CycleOperation::IncrementHorizontalV
                };
             }

             // Now for sprite fetching. We do this for 8 sequences, (we can only render up to 8 sprites)
             scanline[257] = CycleOperation::EqualizeHorizontalVT;
             for x in 0..8 {
                let offset = 256 + 8 * x;
                scanline[offset + 2] = CycleOperation::UnusedNameTableAccess;
                scanline[offset + 3] = CycleOperation::IgnoredNameTableAccess;
                scanline[offset + 5] = CycleOperation::SpriteLsb;
                scanline[offset + 7] = CycleOperation::SpriteMsb;
            }
            
            // First two tiles on the next scanline
            for x in 0..2 {
                let offset = 320 + 8 * x;
                scanline[offset + 1] = CycleOperation::NameTableAccess;
                scanline[offset + 3] = CycleOperation::AttributeTableAccess;
                scanline[offset + 5] = CycleOperation::BackgroundLsb;
                scanline[offset + 7] = CycleOperation::BackgroundMsb;
                scanline[offset + 8] = CycleOperation::IncrementHorizontalV;
            }

            // Unused name table fetches
            scanline[338] = CycleOperation::UnusedNameTableAccess;
            scanline[340] = CycleOperation::IgnoredNameTableAccess;

        }

        frame_operations[240][0] = CycleOperation::BackgroundLsbit;
        frame_operations[241][1] = CycleOperation::SetVblank;
        // Pre-renders
        let mut prerender_scanline = frame_operations[261];
        prerender_scanline[1] = CycleOperation::ClearVblankAndSpriteZero;
        // TODO (does this mean we don't do sprite rendering on the prerender scanline?)
        // for x in 280..=304 {
        //     prerender_scanline[x] = CycleOperation::EqualizeVerticalVT;
        // }

        Self {
            memory: PpuMemory::initialize(),
            control: PpuControl::from(0x00),
            loopy_v: 0x0000,
            loopy_t: 0x0000,
            fine_x: 0x00,
            loopy_w: WriteToggle::First,
            frame_operations: frame_operations,
            frame_index: (261, 0), // Starts on the pre-render line
        }
    }

    fn read_or_write_to_register_detected(&mut self, register: PpuRegister) {
        match register {
            PpuRegister::Control(d) => {
                // t: ...GH.. ........ <- d: ......GH
                // Bit shift left 10 times and clear bits 11 and 12 in t
                self.loopy_t = (((d & 0x03) as u16) << 10) | (self.loopy_t & 0xF3FF);
            },
            PpuRegister::Status(d) => {
                self.loopy_w = WriteToggle::First;
            },
            PpuRegister::Scroll(d) => {
                let fine_x = d & 0x07;
                let upper_five = (d & 0xF8) >> 3;
                match self.loopy_w {
                    // t: ....... ...ABCDE <- d: ABCDE...
                    // x:              FGH <- d: .....FGH
                    // w:                  <- 1
                    WriteToggle::First => {
                        self.loopy_t = (self.loopy_t & 0xFFE0) | (upper_five as u16);
                        self.fine_x = fine_x;
                        self.loopy_w = WriteToggle::Second;
                    },
                    // t: FGH..AB CDE..... <- d: ABCDEFGH
                    // w:                  <- 0
                    WriteToggle::Second => {
                        self.loopy_t = (self.loopy_t & 0x0C1F) | ((fine_x as u16) << 12) | ((upper_five as u16) << 2);
                        self.loopy_w = WriteToggle::First;
                    },
                }
            },
            PpuRegister::Address(d) => {
                let lower_six = d & 0x3F;
                let lower_three = d & 0x07;
                let upper_five = (d & 0xF8) >> 3;
                match self.loopy_w {
                    // t: .CDEFGH ........ <- d: ..CDEFGH
                    //        <unused>     <- d: AB......
                    // t: Z...... ........ <- 0 (bit Z is cleared)
                    // w:                  <- 1
                    WriteToggle::First => {
                        // anding with 0x80 will clear bit 14.
                        self.loopy_t = (self.loopy_t & 0x80FF) | ((lower_six as u16) << 8);
                        self.loopy_w = WriteToggle::Second;
                    },
                    // t: ....... ABCDEFGH <- d: ABCDEFGH
                    // w:                  <- 0
                    //    (wait 1 to 1.5 dots after the write completes)
                    // v: <...all bits...> <- t: <...all bits...>
                    WriteToggle::Second => {
                        self.loopy_t = (self.loopy_t & 0xFF00) | (d as u16);
                        self.loopy_w = WriteToggle::First;
                        // TODO: Should we latch this one cycle?
                        self.loopy_v = self.loopy_t;
                    },
                }
            },
            PpuRegister::Data(d) => {
                self.loopy_v += self.control.vram_address_increment();
            },
        }
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