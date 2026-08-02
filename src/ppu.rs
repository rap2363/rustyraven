use crate::mappers::mapper::Mapper;
use crate::memory::Segment;
use crate::ppu_registers::{PpuControl, PpuMask, VramIncrement};
use crate::rom::NametableArrangement;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PRERENDER_SCANLINE: usize = 261;
const NUM_SCANLINES: usize = 262;
const NUM_DOTS: usize = 341;
const BG_PALETTE_RAM: u16 = 0x3F00;
const SPRITE_PALETTE_RAM: u16 = 0x3F10;

// Address space from 0x0000 --> 0xFFFF, but
// with mirrors from 0x4000 onward.
struct PpuMemory {
    // Determines how we map to chr rom (the first 0x2000 bytes).
    mapper: Rc<RefCell<dyn Mapper>>,
    // 0x2000 --> 0x2FFF (with mirrors up to 0x3EFF)
    name_tables: Segment<0x1000>,
    // 0x3F00 --> 0x3F20 (with mirrors up to 0x4000)
    palettes: Segment<0x0040>,
    // 256 bytes of separately addressable memory to store 64 sprites of 4 bytes each.
    oam_data: Segment<0x0100>
}

// Red-Green-Blue
#[derive(Clone, Copy, Debug, Default)]
pub struct Pixel(pub u8, pub u8, pub u8);

#[derive(Copy, Clone)]
enum SpritePriority {
    BehindBackground,
    InFrontOfBackground,
}

#[derive(Clone, Copy)]
struct SpritePixel(Pixel, SpritePriority, bool); // bool encodes whether the pixel is for a sprite 0 pixel or not.

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

fn get_system_color(color_index: u8) -> Pixel {
    SYSTEM_PALETTE_COLORS[color_index as usize]
}

impl PpuMemory {
     pub fn initialize(mapper: Rc<RefCell<dyn Mapper>>) -> Self {
        Self {
            mapper: mapper,
            name_tables: Segment::<0x1000>::initialize(),
            palettes: Segment::<0x0040>::initialize(),
            oam_data: Segment::<0x0100>::initialize(),
        }
    }

    fn get_mirrored_nametable_address(address: u16, nt_arrangement: NametableArrangement) -> u16 {
        let mut nametable_address = ((address - 0x2000) % 0x1000);
        match nt_arrangement {
            // A: [0x2000, 0x23FF]
            // A: [0x2400, 0x27FF] (mirrored)
            // B: [0x2800, 0x2BFF]
            // B: [0x2C00, 0x2FFF] (mirrored)
            NametableArrangement::HorizontallyMirrored => {
                nametable_address &= 0xFBFF; 
            },
            // A: [0x2000, 0x23FF]
            // B: [0x2400, 0x27FF]
            // A: [0x2800, 0x2BFF] (mirrored)
            // B: [0x2C00, 0x2FFF] (mirrored)
            NametableArrangement::VerticallyMirrored => {
                nametable_address &= 0xF7FF; 
            },
            // A: [0x2000, 0x23FF]
            // A: [0x2400, 0x27FF] (mirrored)
            // A: [0x2800, 0x2BFF] (mirrored)
            // A: [0x2C00, 0x2FFF] (mirrored)
            NametableArrangement::SingleScreenLo => {
                nametable_address &= 0xF3FF; 
            },
            // B: [0x2000, 0x23FF] (mirrored)
            // B: [0x2400, 0x27FF]
            // B: [0x2800, 0x2BFF] (mirrored)
            // B: [0x2C00, 0x2FFF] (mirrored)
            NametableArrangement::SingleScreenHi => {
                nametable_address &= 0xF0FF; 
                nametable_address |= 0x0400; // Just set the fourth bit.
            },
        }
        nametable_address
    }
    pub fn write_byte(&mut self, address: u16, value: u8) {
        // First memory map modulo 0x4000.
        let address = address % 0x4000;
        if address < 0x2000 {
            // Pattern Tables (CHR-ROM) handled by the mapper.
            self.mapper.borrow_mut().write_chr_rom_byte(address, value);
        } else if address < BG_PALETTE_RAM {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            // Now we mirror according to our current arrangement.
            let nametable_address = Self::get_mirrored_nametable_address(
                address, 
                self.mapper.borrow().get_nametable_arrangement()
            );
            self.name_tables.write_byte(nametable_address as usize, value);
        } else {
            // Palette Memory
            let mut palette_memory_address = (address - BG_PALETTE_RAM) % 0x20;
            if palette_memory_address & 0x0013 == 0x0010 {
                palette_memory_address &= 0xFFEF;
            }
            self.palettes.write_byte(palette_memory_address as usize, value);
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
            // Pattern Tables (CHR ROM) handled by the mapper.
            self.mapper.borrow().read_chr_rom_byte(address)
        } else if address < BG_PALETTE_RAM {
            // Name Tables (mirrors from 0x3000 -> 0x3F00)
            // Now we mirror according to our current arrangement.
            let nametable_address = Self::get_mirrored_nametable_address(
                address, 
                self.mapper.borrow().get_nametable_arrangement()
            );
            self.name_tables.read_byte(nametable_address as usize)
        } else {
            // Palette Memory
            let palette_memory_address = (address - BG_PALETTE_RAM) % 0x20;
            self.palettes.read_byte(palette_memory_address as usize)
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

    fn get_line_sprites(&self, scanline_y: u8) -> Vec<Sprite> {
        // Creates a sprites for the line.
        let mut sprites_on_scanline = Vec::with_capacity(8);
        for i in (0..0x100).step_by(4) {
            let index = i as usize;
            let oam_y = self.oam_data.read_byte(index) as u16; // Sometimes a sprite can be written to 0xFF.
            let top_y = oam_y + 1; // Add 1 because sprite positions are always written with a decrement.
            let x_pos = self.oam_data.read_byte(index + 3);

            if (scanline_y as u16) >= top_y && (scanline_y as u16) < top_y + 8 && top_y <= 240 {
                // From NES dev wiki: 
                // X-scroll values of $F9-FF results in parts of the sprite to be past the right edge of the screen, thus invisible. 
                if x_pos < 0xF9 {
                    sprites_on_scanline.push(
                        Sprite::from(&[
                            self.oam_data.read_byte(index),
                            self.oam_data.read_byte(index + 1),
                            self.oam_data.read_byte(index + 2),
                            self.oam_data.read_byte(index + 3),
                        ],
                        index == 0,
                    ));
                }
            }
        }

        sprites_on_scanline
    }

    fn create_sprite_line_pixels(&self, sprites_on_scanline: Vec<Sprite>, sprite_pattern_table: u16, scanline_y: u8) -> Vec<Option<SpritePixel>> {
        // 256 pixel initialization.
        let mut sprite_pixels = vec![None; 0x100];

        // Iterate through each sprite and update the sprite pixels accordingly. Iterate in reverse so that
        // sprites earlier in the vec take priority and overwrite later ones.
        for sprite in sprites_on_scanline.into_iter().rev() {
            let row = scanline_y - (sprite.top_y + 1);
            let y = if sprite.v_flip { 7 - row } else { row } as u16;
            let hi_pattern_table_address = (sprite_pattern_table << 12) | ((sprite.tile_index as u16) << 4) | 0x08 | y;
            let lo_pattern_table_address = (sprite_pattern_table << 12) | ((sprite.tile_index as u16) << 4) | y;
            let pattern_table_byte_hi = self.read_byte(hi_pattern_table_address);
            let pattern_table_byte_lo = self.read_byte(lo_pattern_table_address);

            // Don't go over the right edge.
            let start = sprite.left_x as u16;
            for i in start..=(start + 7).min(0xFF) {
                let offset = i - start;
                let shift = if sprite.h_flip { offset } else { 7 - offset };
                let hi = (pattern_table_byte_hi >> shift) & 0x01;
                let lo = (pattern_table_byte_lo >> shift) & 0x01;
                // Skip transparent pixels.
                if hi == 0x00 && lo == 0x00 {
                    continue;
                }
                let palette = sprite.palette_bits & 0x03;
                let value = (((hi << 1) | lo) & 0x03) as u16;

                // Color index is a 6-bit index into system colors.
                let color_index = (self.read_byte(SPRITE_PALETTE_RAM | ((palette as u16) << 2) | (value as u16))) & 0x3F;
                let pixel = get_system_color(color_index);
                sprite_pixels[i as usize] = Some(SpritePixel(pixel, sprite.priority, sprite.is_sprite_zero()));
            }
        }
        sprite_pixels
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
    SpriteEvaluation,
    SpriteZeroClear,
    SpriteOverflowClear,
}

struct LatchedDataBuffer(u8);

impl LatchedDataBuffer {
    fn read_and_set(&mut self, new_data: u8) -> u8 {
        let data = self.0;
        // Shift in the new data.
        self.0 = new_data;
        data
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
        std::mem::swap(&mut self.front, &mut self.back);
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

struct Sprite {
    left_x: u8,
    top_y: u8,
    tile_index: u8,
    palette_bits: u8,
    priority: SpritePriority,
    h_flip: bool,
    v_flip: bool,
    is_sprite_zero: bool,
}

impl Sprite {
    fn from(bytes: &[u8; 4], is_sprite_zero: bool) -> Self {
        let [top_y, tile_index, attributes, left_x] = *bytes;

        let palette_bits = attributes & 0x03;
        let priority = if (attributes >> 5) & 0x01 == 0x01 { SpritePriority::BehindBackground } else { SpritePriority::InFrontOfBackground };
        let h_flip = (attributes >> 6) & 0x01 == 0x01;
        let v_flip = (attributes >> 7) & 0x01 == 0x01;

        Self { left_x, top_y, tile_index, palette_bits, priority, h_flip, v_flip, is_sprite_zero: is_sprite_zero }
    }

    fn is_sprite_zero(&self) -> bool {
        self.is_sprite_zero
    }
}

pub struct Ppu {
    memory: PpuMemory,
    control: PpuControl,
    mask: PpuMask,
    oam_address: u8,
    dma_complete: Cell<bool>,
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
    line_sprite_pixels: Vec<Option<SpritePixel>>,
}

enum WriteToggle {
    First,
    Second,
}

impl Ppu {
    pub fn initialize(mapper: Rc<RefCell<dyn Mapper>>) -> Self {
        let mut frame_operations: Vec<[Vec<CycleOperation>; NUM_DOTS]> = (0..NUM_SCANLINES).map(|_| std::array::from_fn(|_| Vec::new())).collect();
        // Frame diagram: https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
        // Visible lines + Prerender line.
        for row_index in (0..=239).into_iter().chain(PRERENDER_SCANLINE..PRERENDER_SCANLINE + 1) {
            let scanline: &mut [Vec<CycleOperation>; NUM_DOTS] = &mut frame_operations[row_index];
            // Technically this is done in the "previous" line, but this is so fast that we can just do this in the first cycle and use the evalution.
            if row_index < 240 {
                scanline[0].push(CycleOperation::SpriteEvaluation);
            }
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
        prerender_scanline[1] = vec![CycleOperation::ClearVblank, CycleOperation::SpriteZeroClear, CycleOperation::SpriteOverflowClear];
        for x in 280..=304 {
            prerender_scanline[x].push(CycleOperation::EqualizeVerticalVT);
        }

        Self {
            memory: PpuMemory::initialize(mapper),
            control: PpuControl::from(0x00),
            mask: PpuMask::from(0x00),
            oam_address: 0x00,
            dma_complete: Cell::new(false),
            ppu_data: LatchedDataBuffer(0x00),
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
            line_sprite_pixels: vec![None; 0x100],
        }
    }

    pub fn write_chr_rom_data(&mut self, data: &[u8]) {
        self.memory.write_bytes(0x00, data);
    }

    pub fn vblank(&self) -> bool {
        self.vblank
    }

    pub fn nmi(&self) -> bool {
        self.nmi && self.vblank
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
                self.memory.oam_data.write_byte(self.oam_address as usize, data);
                // Increment OAM after the write.
                self.oam_address = self.oam_address.wrapping_add(1);
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
                        self.loopy_v = self.loopy_t;
                    },
                }
            },
            // PPU Data
            0x2007 => {
                // Write the data to memory
                self.memory.write_byte(self.loopy_v, data);
                self.increment_vram();
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
                let s_bit = if self.sprite_zero_hit { 0x40 } else { 0x00 };
                let o_bit = if self.sprite_overflow { 0x20 } else { 0x00 };

                // Clear the VBlank flag.
                self.vblank = false;
                // Reset the write latch
                self.loopy_w = WriteToggle::First;
                v_bit | s_bit | o_bit
            },
            // OAM Address 
            0x2003 => self.oam_address,
            // OAM Data
            0x2004 => self.memory.oam_data.read_byte(self.oam_address as usize),
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
                let data = if (self.loopy_v & 0x3FFF) >= 0x3400 {
                    let d = self.memory.read_byte(self.loopy_v);
                    self.ppu_data.read_and_set(d);
                    d
                } else {
                    self.ppu_data.read_and_set(self.memory.read_byte(self.loopy_v))
                };
                self.increment_vram();
                data
            },
            _ => panic!("Unimplemented address read from: 0x{:4X}", address),
        }
    }

    fn increment_vram(&mut self) {
        let inc = match self.control.vram_address_increment() {
            VramIncrement::CoarseX => 1,
            VramIncrement::Y => 32,
        };
        self.loopy_v = self.loopy_v.wrapping_add(inc);
    }
    pub fn dma(&mut self, dma_bytes: &[u8]) {
        // Write all 256 bytes into oam_data.
        self.memory.oam_data.write_bytes(0x00, dma_bytes);
        // Flag that we just DMA'ed (so we can increment the CPU cycle count).
        self.dma_complete.set(true);
    }

    pub fn dma_flag(&self) -> bool {
        let ret = self.dma_complete.get();
        self.dma_complete.set(false);
        ret
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

    pub fn get_image(&self) -> Option<Vec<Pixel>> {
        self.image_buffer.front().filter(|pixels| pixels.len() == 256 * 240)
    }

    pub fn execute_cycle(&mut self) {
        let (scanline, dot) = self.frame_index;
        for i in 0..self.frame_operations[scanline][dot].len() {
            // Execute the operation
            let operation = self.frame_operations[scanline][dot][i];
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
                if self.frame_index.0 < 240 && self.frame_index.1 < 257 {
                    // Get a background pixel line from the high and low bytes of the background
                    let mut bg_pixel_line = [(0x00, Pixel::default()); 8];
                    for i in 0..8 {
                        let shift: u8 = 15 - self.fine_x - i;
                        let hi = self.pattern_byte_sr_hi.bit(shift);
                        let lo = self.pattern_byte_sr_lo.bit(shift);
                        // Current tile's palette, use next byte (lo) if fine_x bleeds over
                        let palette = if self.fine_x + i <= 7 {
                            self.attribute_byte_sr.hi()   // current tile
                        } else {
                            self.attribute_byte_sr.lo()   // neighbor tile
                        } & 0x03;
                        let value = (((hi << 1) | lo) & 0x03) as u16;

                        // let color_index = (self.memory.read_byte(BG_PALETTE_RAM | ((palette as u16) << 2) | (value as u16))) & 0x3F;
                        // bg_pixel_line.push((value, get_system_color(color_index)));
                        // Get the right color value from Palette RAM.
                        // Color index is a 6-bit index into system colors.
                        if value == 0x00 {
                            let transparent_index = self.memory.read_byte(BG_PALETTE_RAM);
                            bg_pixel_line[i as usize] = (value, get_system_color(transparent_index));
                        } else {
                            // Color index is a 6-bit index into system colors.
                            let color_index = (self.memory.read_byte(BG_PALETTE_RAM | ((palette as u16) << 2) | (value as u16))) & 0x3F;
                            bg_pixel_line[i as usize] = (value, get_system_color(color_index));
                        }
                    }

                    // Get a sprite pixel line as well.
                    let mut sprite_pixel_line = [None; 8];
                    for i in 0..8 {
                        sprite_pixel_line[i] = self.line_sprite_pixels[self.frame_index.1 - (8 - i)];
                    }

                    // Now combine the pixels into a single pixel_line to push.
                    let mut pixel_line = [Pixel::default(); 8];
                    for i in 0..8 {
                        let (bg_value, bg_color) = bg_pixel_line[i];
                        pixel_line[i] = match sprite_pixel_line[i] {
                            Some(SpritePixel(sp_color, SpritePriority::InFrontOfBackground, is_sprite_zero)) => {
                                if is_sprite_zero {
                                    self.sprite_zero_hit = true;
                                }
                                sp_color
                            },
                            Some(SpritePixel(sp_color, SpritePriority::BehindBackground, is_sprite_zero)) => {
                                if bg_value == 0 {
                                    sp_color
                                } else {
                                    if is_sprite_zero {
                                        self.sprite_zero_hit = true;
                                    }
                                    bg_color
                                }
                            },
                            _ => bg_color,
                        };
                    }

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
            CycleOperation::SpriteEvaluation => {
                let scanline_y = self.frame_index.0.try_into().expect("Frame Index called from here shouldn't exceed 240");
                let sprites = self.memory.get_line_sprites(scanline_y);
                if sprites.len() > 8 {
                    // Check and set sprite overflow if we see more than 8 sprites on the line!
                    self.sprite_overflow = true;
                }

                self.line_sprite_pixels = self.memory.create_sprite_line_pixels(
                    sprites,
                    self.control.sprite_pattern_table_address() as u16, 
                    scanline_y,
                );
            }
            CycleOperation::ClearVblank => {
                self.vblank = false;
            },
            CycleOperation::SetVblank => {
                self.vblank = true;
                self.image_buffer.swap();
            },
            CycleOperation::EqualizeHorizontalVT => {
                // Copy over the horizontal bits
                // v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
                self.loopy_v = (self.loopy_v & 0xFBE0) | (self.loopy_t & 0x041F)
            },
            CycleOperation::EqualizeVerticalVT => {
                // Copy over the vertical bits.
                // v: GHIA.BC DEF..... <- t: GHIA.BC DEF.....
                self.loopy_v = (self.loopy_v & 0x041F) | (self.loopy_t & 0xFBE0)
            },
            CycleOperation::SpriteZeroClear => {
                self.sprite_zero_hit = false;
            },
            CycleOperation::SpriteOverflowClear => {
                self.sprite_overflow = false;
            },
            CycleOperation::IgnoredNameTableAccess => {},
        }
    }
}
