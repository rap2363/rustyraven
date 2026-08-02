use crate::memory::Segment;
use crate::mappers::mapper::Mapper;
use crate::rom::{NametableArrangement, NesRom};

// Mapper 0 is a really simple identity mapping between CPU/PPU ROM and the appropriate bank data.
//
// Schematic:
// 
// CPU:
//
// 0x8000______________
// |                  |-----------> mapper.prg_rom_data
// |      (16 kb)     |
// |                  |
// 0xC000_____________|
// |                  |
// |      (16 kb)     |
// |                  |
// |__________________|
//
// PPU
//
// 0x0000______________
// |                  |-----------> mapper.chr_rom_data
// |______(8 kb)______|
//
pub struct Mapper0 {
    chr_rom_data: Segment<0x2000>,
    prg_rom_data: Segment<0x8000>,
    nametable_arrangement: NametableArrangement,
}

impl Mapper0 {
    pub fn from(rom: &NesRom) -> Self {
        let mut chr_rom_data = Segment::<0x2000>::initialize();
        let mut prg_rom_data = Segment::<0x8000>::initialize();
        let nametable_arrangement = rom.name_table_arrangement;

        if rom.chr_rom_data.len() == 0x2000 {
            chr_rom_data.write_bytes(0x0000, &rom.chr_rom_data);
        } else {
            panic!("Malformed chr-rom data for Mapper 0!")
        }

        if rom.prg_rom_data.len() == 0x4000 {
            // We replicate the PRG ROM data across the entire 0x8000 segment.
            prg_rom_data.write_bytes(0x0000, &rom.prg_rom_data);
            prg_rom_data.write_bytes(0x4000, &rom.prg_rom_data);
        } else if rom.prg_rom_data.len() == 0x8000 {
            // Write all of the PRG ROM data into the segment directly.
            prg_rom_data.write_bytes(0x0000, &rom.prg_rom_data);
        } else {
            panic!("Malformed PRG ROM data for Mapper 0!")
        }

        Self { chr_rom_data, prg_rom_data, nametable_arrangement }
    }

    pub fn test_initialize() -> Self {
        let chr_rom_data = Segment::<0x2000>::initialize();
        let prg_rom_data = Segment::<0x8000>::initialize();
        let nametable_arrangement = NametableArrangement::HorizontallyMirrored;

        Self { chr_rom_data, prg_rom_data, nametable_arrangement }
    }
}

impl Mapper for Mapper0 {
    fn read_prg_rom_byte(&self, address: u16) -> u8 {
        let shifted_address = (address - 0x8000) as usize;
        self.prg_rom_data.read_byte(shifted_address)
    }

    fn read_chr_rom_byte(&self, address: u16) -> u8 {
        self.chr_rom_data.read_byte(address as usize)
    }

    // Ignore, we don't do any bank switching for Mapper 0.
    fn write_prg_rom_byte(&mut self, _address: u16, _value: u8) {}

    fn write_chr_rom_byte(&mut self, address: u16, value: u8) {
        self.chr_rom_data.write_byte(address as usize, value);
    }

    fn get_nametable_arrangement(&self) -> NametableArrangement {
        self.nametable_arrangement
    }
}
