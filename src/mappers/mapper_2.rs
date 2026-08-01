use crate::memory::Segment;
use crate::mappers::mapper::Mapper;
use crate::rom::{NametableArrangement, NesRom};

// Mapper 2 has fixed CHR ROM, but allows for some switching of the PRG ROM banks.
// Link: https://www.nesdev.org/wiki/UxROM
// Specifically, ROM capacity is 256kb (or 4096 kb apparently, but we're not implementing that here.)
// With 256 kb, we have 32 banks to swap between, where the *last* bank is fixed to the 64th bank always.
// We use a bank select by listening for writes like so:
//
// Bank select ($8000-$FFFF)
// 
// 7  bit  0
// ---- ----
// xxxx pPPP
//     ||||
//     ++++- Select 16 KB PRG ROM bank for CPU $8000-$BFFF
//         (UNROM uses bits 2-0; UOROM uses bits 3-0)
//
// So effectively, we keep track of a "current" bank using the pPPP bits to index into a
// an array of 8 or 16 banks. (letting us switch between 128kb or 256kb of memory).
//
// For simplicity, the implementation below just stores the bank index from the full 8 bit write.
//
//
// Schematic:
// 
// CPU:
//
// 0x8000______________
// |                  |-----------> mapper.bank[current_bank]
// |      (16 kb)     |
// |                  |
// 0xC000_____________|
// |                  |-----------> mapper.fixed_bank
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
pub struct Mapper2 {
    chr_rom_data: Segment<0x2000>,
    prg_rom_banks: Vec<Segment<0x4000>>,
    current_bank: u8,
    fixed_bank: Segment<0x4000>,
    nametable_arrangement: NametableArrangement,
}

impl Mapper2 {
    pub fn from(rom: &NesRom) -> Self {
        let mut chr_rom_data = Segment::<0x2000>::initialize();
        let mut prg_rom_banks = vec![];
        let mut fixed_bank = Segment::<0x4000>::initialize();
        let current_bank = 0x00;
        let nametable_arrangement = rom.name_table_arrangement;

        if rom.chr_rom_data.len() == 0x2000 {
            chr_rom_data.write_bytes(0x0000, &rom.chr_rom_data);
        } else {
            // Skip, this is used for PRG RAM.
        }

        for i in (0..rom.prg_rom_size).step_by(0x4000) {
            let mut prg_rom_segment = Segment::<0x4000>::initialize();
            prg_rom_segment.write_bytes(0x0000, &rom.prg_rom_data[i..i+0x4000]);
            prg_rom_banks.push(prg_rom_segment);
        }

        // the very last 0x4000 (16 kb) bank copies over to the fixed bank.
        fixed_bank.write_bytes(0x0000, &rom.prg_rom_data[rom.prg_rom_size-0x4000..rom.prg_rom_size]);

        Self { chr_rom_data, prg_rom_banks, current_bank, fixed_bank, nametable_arrangement }
    }
}

impl Mapper for Mapper2 {
    fn read_prg_rom_byte(&self, address: u16) -> u8 {
        if address < 0xC000 {
            let shifted_address = (address - 0x8000) as usize;
            self.prg_rom_banks[self.current_bank as usize].read_byte(shifted_address)
        } else {
            // Read from the fixed bank
            let shifted_address = (address - 0xC000) as usize;
            self.fixed_bank.read_byte(shifted_address)
        }
    }

    fn read_chr_rom_byte(&self, address: u16) -> u8 {
        self.chr_rom_data.read_byte(address as usize)
    }

    // Read the byte value and store as our current bank.
    fn write_prg_rom_byte(&mut self, _address: u16, value: u8) {
        self.current_bank = value & 0x0F;
    }

    fn write_chr_rom_byte(&mut self, address: u16, value: u8) {
        self.chr_rom_data.write_byte(address as usize, value);
    }

    fn get_nametable_arrangement(&self) -> NametableArrangement {
        self.nametable_arrangement
    }
}
