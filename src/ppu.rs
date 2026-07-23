use crate::memory::Segment;
use crate::ppu_registers::{PpuControl, PpuMask, PpuStatus, VramIncrement};
use std::cell::Cell;

const PRERENDER_SCANLINE: usize = 261;
const NUM_SCANLINES: usize = 262;
const NUM_DOTS: usize = 341;
const PALETTE_RAM: u16 = 0x3F00;

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

// Red-Green-Blue
#[derive(Clone, Copy)]
pub struct Pixel(pub u8, pub u8, pub u8);

// I stole this from myself (WhiteRaven)
const SYSTEM_PALETTE_COLORS: [Pixel; 64] = [
    Pixel(0x75, 0x75, 0x75), // 0x00
    Pixel(0x27, 0x1B, 0x8F), // 0x01
    Pixel(0x00, 0x00, 0xAB), // 0x02
    Pixel(0x47, 0x00, 0x9F), // 0x03
    Pixel(0x8F, 0x00, 0x77), // 0x04
    Pixel(0xAB, 0x00, 0x13), // 0x05
    Pixel(0xA7, 0x00, 0x00), // 0x06
    Pixel(0x7F, 0x0B, 0x00), // 0x07
    Pixel(0x43, 0x2F, 0x00), // 0x08
    Pixel(0x00, 0x47, 0x00), // 0x09
    Pixel(0x00, 0x51, 0x00), // 0x0A
    Pixel(0x00, 0x3F, 0x17), // 0x0B
    Pixel(0x1B, 0x3F, 0x5F), // 0x0C
    Pixel(0x00, 0x00, 0x00), // 0x0D
    Pixel(0x00, 0x00, 0x00), // 0x0E
    Pixel(0x00, 0x00, 0x00), // 0x0F

    Pixel(0xBC, 0xBC, 0xBC), // 0x10
    Pixel(0x00, 0x73, 0xEF), // 0x11
    Pixel(0x23, 0x3B, 0xEF), // 0x12
    Pixel(0x83, 0x00, 0xF3), // 0x13
    Pixel(0xBF, 0x00, 0xBF), // 0x14
    Pixel(0xE7, 0x00, 0x5B), // 0x15
    Pixel(0xDB, 0x2B, 0x00), // 0x16
    Pixel(0xCB, 0x4F, 0x0F), // 0x17
    Pixel(0x8B, 0x73, 0x00), // 0x18
    Pixel(0x00, 0x97, 0x00), // 0x19
    Pixel(0x00, 0xAB, 0x00), // 0x1A
    Pixel(0x00, 0x93, 0x3B), // 0x1B
    Pixel(0x00, 0x83, 0x8B), // 0x1C
    Pixel(0x00, 0x00, 0x00), // 0x1D
    Pixel(0x00, 0x00, 0x00), // 0x1E
    Pixel(0x00, 0x00, 0x00), // 0x1F

    Pixel(0xFF, 0xFF, 0xFF), // 0x20
    Pixel(0x3F, 0xBF, 0xFF), // 0x21
    Pixel(0x5F, 0x97, 0xFF), // 0x22
    Pixel(0xA7, 0x8B, 0xFD), // 0x23
    Pixel(0xF7, 0x7B, 0xFF), // 0x24
    Pixel(0xFF, 0x77, 0xB7), // 0x25
    Pixel(0xFF, 0x77, 0x63), // 0x26
    Pixel(0xFF, 0x9B, 0x3B), // 0x27
    Pixel(0xF3, 0xBF, 0x3F), // 0x28
    Pixel(0x83, 0xD3, 0x13), // 0x29
    Pixel(0x4F, 0xDF, 0x4B), // 0x2A
    Pixel(0x58, 0xF8, 0x98), // 0x2B
    Pixel(0x00, 0xEB, 0xDB), // 0x2C
    Pixel(0x00, 0x00, 0x00), // 0x2D
    Pixel(0x00, 0x00, 0x00), // 0x2E
    Pixel(0x00, 0x00, 0x00), // 0x2F

    Pixel(0xFF, 0xFF, 0xFF), // 0x30
    Pixel(0xAB, 0xE7, 0xFF), // 0x31
    Pixel(0xC7, 0xD7, 0xFF), // 0x32
    Pixel(0xD7, 0xCB, 0xFF), // 0x33
    Pixel(0xFF, 0xC7, 0xFF), // 0x34
    Pixel(0xFF, 0xC7, 0xDB), // 0x35
    Pixel(0xFF, 0xBF, 0xB3), // 0x36
    Pixel(0xFF, 0xDB, 0xAB), // 0x37
    Pixel(0xFF, 0xE7, 0xA3), // 0x38
    Pixel(0xE3, 0xFF, 0xA3), // 0x39
    Pixel(0xAB, 0xF3, 0xBF), // 0x3A
    Pixel(0xB3, 0xFF, 0xCF), // 0x3B
    Pixel(0x9F, 0xFF, 0xF3), // 0x3C
    Pixel(0x00, 0x00, 0x00), // 0x3D
    Pixel(0x00, 0x00, 0x00), // 0x3E
    Pixel(0x00, 0x00, 0x00), // 0x3F
];

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
        } else if address < PALETTE_RAM {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            let name_table_byte = (address - 0x2000) % 0x1000;
            self.name_tables.write_byte(name_table_byte as usize, value);
        } else {
            // Pallete Memory
            let pallete_memory_address = (address - PALETTE_RAM) % 0x20;
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
        } else if address < PALETTE_RAM {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            let name_table_byte = (address - 0x2000) % 0x1000;
            self.name_tables.read_byte(name_table_byte as usize)
        } else {
            // Pallete Memory
            let pallete_memory_address = (address - PALETTE_RAM) % 0x20;
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
#[derive(Copy, Clone, Debug)]
enum CycleOperation {
    NameTableAccess,
    UnusedNameTableAccess,
    IgnoredNameTableAccess,
    AttributeTableAccess,
    BackgroundLsb,
    BackgroundMsb,
    IncrementHorizontalV,
    IncrementVerticalV,
    EqualizeHorizontalVT,
    EqualizeVerticalVT,
    SetVblank,
    ClearVblank,
    SpriteZeroCheck,
    SpriteOverflowCheck,
    SpriteLsb,
    SpriteMsb,
}

struct LatchedDataBuffer {
    buffer: u8,
    data: u8,
}

impl LatchedDataBuffer {
    fn initialize() -> Self {
        Self {
            buffer: 0x00,
            data: 0x00,
        }
    }

    fn read(&mut self) -> u8 {
        let data_to_return = self.buffer;
        self.buffer = self.data;
        data_to_return
    }

    fn write(&mut self, data: u8) {
        self.data = data;
    }
}

// Used for storing/writing the current image, this structure is used to return 
// an owned copy of a fully "written" image while we continue to write to the other.
struct DoubleBuffer {
    front: Vec<Pixel>,
    back: Vec<Pixel>,
    ready: Cell<bool>,
}

impl DoubleBuffer {
    fn initialize() -> Self {
        Self {
            front: Vec::with_capacity(256 * 240),
            back: Vec::with_capacity(256 * 240),
            ready: Cell::new(false),
        }
    }

    fn front(&self) -> Option<Vec<Pixel>> {
        if self.ready.get() { 
            self.ready.set(false);
            Some(self.front.clone()) 
        } else { 
            None 
        }
    }

    fn back(&mut self) -> &mut Vec<Pixel> {
        &mut self.back
    }

    fn swap(&mut self) {
        self.front = self.back.clone();
        self.back.clear();
        self.ready.set(true);
    }
}

struct ShiftRegister(u16);

impl ShiftRegister {

    fn initialize() -> Self {
        Self(0x0000)
    }

    fn push(&mut self, byte: u8) {
        self.0 = (self.0 << 8) | (byte as u16);
    }

    fn hi(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    fn lo(&self) -> u8 {
        self.0 as u8
    }

    fn bit(&self, n: u8) -> u8 {
        ((self.0 >> n) & 0x01) as u8
    }
}

pub struct Ppu {
    memory: PpuMemory,
    control: PpuControl,
    mask: PpuMask,
    oam_address: u8,
    oam_data: u8,
    ppu_data: LatchedDataBuffer,
    loopy_v: u16,
    loopy_t: u16,
    fine_x: u8,
    loopy_w: WriteToggle,
    frame_operations: Vec<[Vec<CycleOperation>; NUM_DOTS]>,
    frame_index: (usize, usize), // row, column,
    vblank: bool,
    nmi: bool,
    sprite_overflow: bool,
    sprite_zero_hit: bool,
    name_table_byte: u8,
    attribute_table_group: u8,
    pattern_table_byte_lo: u8,
    pattern_table_byte_hi: u8,
    pattern_byte_sr_hi: ShiftRegister,
    pattern_byte_sr_lo: ShiftRegister,
    attribute_byte_sr: ShiftRegister,
    image_buffer: DoubleBuffer,
}

enum WriteToggle {
    First,
    Second,
}

impl Ppu {
    pub fn initialize() -> Self {
        let mut frame_operations: Vec<[Vec<CycleOperation>; NUM_DOTS]> = (0..NUM_SCANLINES).map(|_| std::array::from_fn(|_| Vec::new())).collect();
        // Frame diagram: https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
        // Visible lines + Prerender line.
        for row_index in (0..=239).into_iter().chain(PRERENDER_SCANLINE..PRERENDER_SCANLINE + 1) {
            let scanline: &mut [Vec<CycleOperation>; NUM_DOTS] = &mut frame_operations[row_index];
            // We do this for 256 pixels in 8-bit increments (so 256 / 8 = 32)
            for x in 0..32 {
                let offset = 8 * x;
                scanline[offset + 1].push(CycleOperation::NameTableAccess);
                scanline[offset + 3].push(CycleOperation::AttributeTableAccess);
                scanline[offset + 5].push(CycleOperation::BackgroundLsb);
                scanline[offset + 7].push(CycleOperation::BackgroundMsb);
                scanline[offset + 8].push(CycleOperation::IncrementHorizontalV);
                if x == 31 {
                    scanline[offset + 8].push(CycleOperation::IncrementVerticalV);
                }
             }

             // Now for sprite fetching. We do this for 8 sequences, (we can only render up to 8 sprites)
             scanline[257] = vec![CycleOperation::EqualizeHorizontalVT];
             for x in 0..8 {
                let offset = 256 + 8 * x;
                scanline[offset + 2].push(CycleOperation::UnusedNameTableAccess);
                scanline[offset + 3].push(CycleOperation::IgnoredNameTableAccess);
                scanline[offset + 5].push(CycleOperation::SpriteLsb);
                scanline[offset + 7].push(CycleOperation::SpriteMsb);
            }
            
            // First two tiles on the next scanline
            for x in 0..2 {
                let offset = 320 + 8 * x;
                scanline[offset + 1].push(CycleOperation::NameTableAccess);
                scanline[offset + 3].push(CycleOperation::AttributeTableAccess);
                scanline[offset + 5].push(CycleOperation::BackgroundLsb);
                scanline[offset + 7].push(CycleOperation::BackgroundMsb);
                scanline[offset + 8].push(CycleOperation::IncrementHorizontalV);
            }

            // Unused name table fetches
            scanline[338].push(CycleOperation::UnusedNameTableAccess);
            scanline[340].push(CycleOperation::IgnoredNameTableAccess);

            // frame_operations[row_index] = scanline;

        }

        frame_operations[241][1].push(CycleOperation::SetVblank);
        // Pre-renders
        let prerender_scanline = &mut frame_operations[PRERENDER_SCANLINE];
        prerender_scanline[1] = vec![CycleOperation::ClearVblank, CycleOperation::SpriteZeroCheck, CycleOperation::SpriteOverflowCheck];
        for x in 280..=304 {
            prerender_scanline[x].push(CycleOperation::EqualizeVerticalVT);
        }

        Self {
            memory: PpuMemory::initialize(),
            control: PpuControl::from(0x00),
            mask: PpuMask::from(0x00),
            oam_address: 0x00,
            oam_data: 0x00,
            ppu_data: LatchedDataBuffer::initialize(),
            loopy_v: 0x0000,
            loopy_t: 0x0000,
            fine_x: 0x00,
            loopy_w: WriteToggle::First,
            frame_operations: frame_operations,
            frame_index: (PRERENDER_SCANLINE, 0), // Starts on the pre-render line
            vblank: false,
            nmi: false,
            sprite_overflow: false,
            sprite_zero_hit: false,
            name_table_byte: 0x0000,
            attribute_table_group: 0x00,
            pattern_table_byte_lo: 0x00,
            pattern_table_byte_hi: 0x00,
            pattern_byte_sr_hi: ShiftRegister::initialize(),
            pattern_byte_sr_lo: ShiftRegister::initialize(),
            attribute_byte_sr: ShiftRegister::initialize(),
            image_buffer: DoubleBuffer::initialize(),
        }
    }

    pub fn write_chr_rom_data(&mut self, data: &[u8]) {
        self.memory.write_bytes(0x00, data);
    }

    pub fn vblank(&self) -> bool {
        self.vblank
    }

    pub fn nmi(&self) -> bool {
        self.nmi
    }

    fn rendering_enabled(&self) -> bool {
        self.mask.bg_enabled() || self.mask.sprites_enabled()
    }

    pub fn write_io_register(&mut self, address: u16, data: u8) {
        match address {
            // PPU Control
            0x2000 => {
                self.control = PpuControl::from(data);
                self.nmi = data & 0x80 == 0x80;
                // t: ...GH.. ........ <- d: ......GH
                // Bit shift left 10 times and clear bits 11 and 12 in t
                self.loopy_t = (((self.control.into() & 0x03) as u16) << 10) | (self.loopy_t & 0xF3FF);
            },
            // PPU Mask
            0x2001 => {
                self.mask = PpuMask::from(data);
            },
            // PPU Status
            0x2002 => {
                // Ignore these writes, but log anyway.
                println!("CPU write to PPU Status register detected: 0x{:4X}, 0x{:2X}", address, data);
            },
            // OAM Address 
            0x2003 => {
                self.oam_address = data;
            },
            // OAM Data
            0x2004 => {
                self.oam_data = data;
            },
            // PPU Scroll
            0x2005 => {
                let ppu_scroll = data;
                let fine_x = ppu_scroll & 0x07;
                let upper_five = (ppu_scroll & 0xF8) >> 3;
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
                        self.loopy_t = (self.loopy_t & 0x0C1F) | ((fine_x as u16) << 12) | ((upper_five as u16) << 5);
                        self.loopy_w = WriteToggle::First;
                    },
                }
            },
            // PPU Address
            0x2006 => {
                let ppu_address = data;
                let lower_six = ppu_address & 0x3F;
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
                        self.loopy_t = (self.loopy_t & 0xFF00) | (ppu_address as u16);
                        self.loopy_w = WriteToggle::First;
                        // TODO: Should we latch this one cycle?
                        self.loopy_v = self.loopy_t;
                    },
                }
            },
            // PPU Data
            0x2007 => {
                // Write the data to memory
                self.memory.write_byte(self.loopy_v, data);
                // Write the byte to our latch too
                self.ppu_data.write(data);
                // Increment VRAM address
                let inc = match self.control.vram_address_increment() {
                    VramIncrement::CoarseX => 1,
                    VramIncrement::Y => 32,
                };
                self.loopy_v = self.loopy_v.wrapping_add(inc);
            },
            _ => panic!("Unimplemented address written to: 0x{:4X}, 0x{:2X}", address, data),
        }
    }

    pub fn read_io_register(&mut self, address: u16) -> u8 {
        match address {
            // PPU Control
            0x2000 => self.control.into(),
            // PPU Mask
            0x2001 => self.mask.into(),
            // PPU Status
            0x2002 => {
                // Build this byte up from our status flags.
                // 7  bit  0
                // ---- ----
                // VSOx xxxx
                // |||| ||||
                // |||+-++++- (PPU open bus or 2C05 PPU identifier)
                // ||+------- Sprite overflow flag
                // |+-------- Sprite 0 hit flag
                // +--------- Vblank flag, cleared on read.
                let v_bit = if self.vblank { 0x80 } else { 0x00 };
                let s_bit = if self.sprite_overflow { 0x40 } else { 0x00 };
                let z_bit = if self.sprite_zero_hit { 0x20 } else { 0x00 };
                // Clear the VBlank flag.
                self.vblank = false;
                // Reset the write latch
                self.loopy_w = WriteToggle::First;
                v_bit | s_bit | z_bit
            },
            // OAM Address 
            0x2003 => self.oam_address,
            // OAM Data
            0x2004 => self.oam_data,
            // PPU Scroll
            0x2005 => {
                // We shouldn't be reading from this, but return 0x00 if we do.
                // TODO: Should we return something else?
                0x00
            },
            // PPU Address
            0x2006 => {
                // We shouldn't be reading from this, but return 0x00 if we do
                // TODO: Should we return something else?
                self.loopy_v as u8
            },
            // PPU Data
            0x2007 => {
                self.ppu_data.read()
            },
            _ => panic!("Unimplemented address read from: 0x{:4X}", address),
        }
    }

    // See https://www.nesdev.org/wiki/PPU_scrolling for details.
    // This diagram is particularly helpful:
    //
    // yyy NN YYYYY XXXXX
    // ||| || ||||| +++++-- coarse X scroll (what we're adjusting here)
    // ||| || +++++-------- coarse Y scroll
    // ||| ++-------------- nametable select
    // +++----------------- fine Y scroll
    fn increment_coarse_x(&mut self) {
        // If coarse X == 31, we just need to wrap around to 0.
        if self.loopy_v & 0x001F == 31 {
            self.loopy_v &= 0xFFE0;
            // And also switch the horiztonal nametable.
            self.loopy_v ^= 0x0400;
        } else {
            self.loopy_v += 1;
        }
    }

    // See https://www.nesdev.org/wiki/PPU_scrolling for details.
    // This diagram is particularly helpful:
    //
    // yyy NN YYYYY XXXXX
    // ||| || ||||| +++++-- coarse X scroll
    // ||| || +++++-------- coarse Y scroll
    // ||| ++-------------- nametable select
    // +++----------------- fine Y scroll
    fn increment_y(&mut self) {
        // If fine y < 7
        if self.loopy_v & 0x7000 != 0x7000 {
            self.loopy_v += 0x1000; // Increment fine y
        } else {
            self.loopy_v &= 0x0FFF; // Zero out the fine y.
            let mut coarse_y = (self.loopy_v & 0x03E0) >> 5;
            if coarse_y == 29 {
                coarse_y = 0;
                // Switch vertical nametable (we do this 2 rows "early" for some reason)
                self.loopy_v ^= 0x0800;
            } else if coarse_y == 31 {
                coarse_y = 0; // But don't switch the nametable
            } else {
                coarse_y += 1;
            }
            // Now stuff 'er back in there lad
            self.loopy_v = (self.loopy_v & 0xFC1F) | (coarse_y << 5)
        }
    }

    // Returns a number from 0 -> 7 indicating the fine y. Used for picking out the correct 8x1 pixel sliver from our tiles.
    fn fine_y(&self) -> u8 {
        ((self.loopy_v & 0x7000) >> 12) as u8
    }

    fn get_system_color(&self, color_index: u8) -> Pixel {
        SYSTEM_PALETTE_COLORS[color_index as usize]
    }

    pub fn get_image(&self) -> Option<Vec<Pixel>> {
        self.image_buffer.front()
    }

    pub fn execute_cycle(&mut self) {
        let (scanline, dot) = self.frame_index;
        // TODO: Cloning this is kind of ridiculous, but there's not a simple way to call into execute_operation otherwise.
        // Maybe we could RefCell the FrameOperations or something.
        for operation in self.frame_operations[scanline][dot].clone() {
            // Execute the operation
            self.execute_operation(operation);
        }
        // Iterate frame_index
        let next_dot = (dot + 1) % NUM_DOTS;
        let next_scanline = if next_dot == 0 {
            (scanline + 1) % NUM_SCANLINES
        } else {
            scanline
        };
        self.frame_index = (next_scanline, next_dot);
    }

    // The following diagram is helpful to understand how the name table values index into the pattern table.
    // For example, the name table value 0x24 corresponds to 0010 0100 for the "N bits" below.
    //
    // DCBA98 76543210
    // ---------------
    // 0HNNNN NNNNPyyy
    // |||||| |||||+++- T: Fine Y offset, the row number within a tile
    // |||||| ||||+---- P: Bit plane (0: less significant bit; 1: more significant bit)
    // ||++++-++++----- N: Tile number from name table
    // |+-------------- H: Half of pattern table (0: "left"; 1: "right")
    // +--------------- 0: Pattern table is at $0000-$1FFF
    fn execute_operation(&mut self, operation: CycleOperation) {
        // TODO: Is this right?
         // Vblank flag maintenance happens regardless of rendering state. Otherwise skip if we're not rendering.
        match operation {
            CycleOperation::SetVblank | CycleOperation::ClearVblank => {},
            _ => {
                // Skip if we're not rendering.
                if !self.rendering_enabled() {
                    return;
                }
            },
        }

        match operation {
            CycleOperation::NameTableAccess | CycleOperation::UnusedNameTableAccess => {
                let name_table_addr = 0x2000 | (self.loopy_v & 0x0FFF);
                self.name_table_byte = self.memory.read_byte(name_table_addr);
            },
            CycleOperation::AttributeTableAccess => {
                let attribute_table_address = 0x23C0 | (self.loopy_v & 0x0C00) | ((self.loopy_v >> 4) & 0x0038) | ((self.loopy_v >> 2) & 0x0007);
                let attribute_byte = self.memory.read_byte(attribute_table_address);
                // We must also include the attribute table group Quadrant = YX
                let coarse_x = self.loopy_v & 0x001F;
                let coarse_y = (self.loopy_v >> 5) & 0x001F;
                let quadrant = (((coarse_y >> 1) & 0x01) << 1) | ((coarse_x >> 1) & 0x01);
                self.attribute_table_group = ((attribute_byte >> (quadrant * 2)) & 0x03) as u8;
            },
            CycleOperation::BackgroundLsb => {
                // According to diagram above with P = 0.
                let pattern_table_address = ((self.control.bg_pattern_table_half() as u16) << 12) | ((self.name_table_byte as u16) << 4) | (self.fine_y() as u16);
                self.pattern_table_byte_lo = self.memory.read_byte(pattern_table_address);
            },
            CycleOperation::BackgroundMsb => {
                // According to diagram above with P = 1.
                let pattern_table_address = ((self.control.bg_pattern_table_half() as u16) << 12) | ((self.name_table_byte as u16) << 4) | 0x08 | (self.fine_y() as u16);
                self.pattern_table_byte_hi = self.memory.read_byte(pattern_table_address);
            },
            CycleOperation::IncrementHorizontalV => {
                // Incrementing the horizontal VRAM address means building a pixel line and rendering!

                // Get a pixel line from the high and low bytes
                let mut pixel_line = Vec::with_capacity(8);
                for i in 0..8 {
                    let shift = 15 - self.fine_x - i;
                    let hi = self.pattern_byte_sr_hi.bit(shift);
                    let lo = self.pattern_byte_sr_lo.bit(shift);
                    // Current tile's palette, use next byte (lo) if fine_x bleeds over
                    let palette = if self.fine_x + i <= 7 {
                        self.attribute_byte_sr.hi()   // current tile
                    } else {
                        self.attribute_byte_sr.lo()     // neighbor tile
                    } & 0x03;
                    let value = (((hi << 1) | lo) & 0x03) as u16;

                    // Get the right color value from Palette RAM
                    // Color index is a 6-bit index into system colors.
                    let color_index = (self.memory.read_byte(PALETTE_RAM | ((palette as u16) << 2) | (value as u16))) & 0x3F;
                    pixel_line.push(self.get_system_color(color_index));
                }

                if self.frame_index.0 < 240 && self.frame_index.1 < 257 {
                    self.image_buffer.back().extend(pixel_line);
                }

                // Shift the pixels
                self.pattern_byte_sr_hi.push(self.pattern_table_byte_hi);
                self.pattern_byte_sr_lo.push(self.pattern_table_byte_lo);
                self.attribute_byte_sr.push(self.attribute_table_group);
                self.increment_coarse_x();
            },
            CycleOperation::IncrementVerticalV => {
                self.increment_y();
            },
            CycleOperation::ClearVblank => {
                self.vblank = false;
            },
            CycleOperation::SetVblank => {
                self.vblank = true;
                self.image_buffer.swap();
            },
            CycleOperation::EqualizeHorizontalVT => {
                if self.rendering_enabled() {
                    // Copy over the horizontal bits
                    // v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
                    self.loopy_v = (self.loopy_v & 0xFBE0) | (self.loopy_t & 0x041F)
                }
            },
            CycleOperation::EqualizeVerticalVT => {
                if self.rendering_enabled() {
                    // Copy over the vertical bits.
                    // v: GHIA.BC DEF..... <- t: GHIA.BC DEF.....
                    self.loopy_v = (self.loopy_v & 0x041F) | (self.loopy_t & 0xFBE0)
                }
            },
            CycleOperation::SpriteZeroCheck => {
                // Unimplemented for now.
            },
            CycleOperation::SpriteOverflowCheck => {
                // Unimplemented for now.
            },
            CycleOperation::SpriteLsb => {
                // Unimplemented for now.
            },
            CycleOperation::SpriteMsb => {
                // Unimplemented for now.
            },
            CycleOperation::IgnoredNameTableAccess => {},
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

    #[test]
    fn test_initialize_ppu() {
        let ppu = Ppu::initialize();
        println!("{:?}", ppu.frame_operations[0]);
    }
}