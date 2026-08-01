use std::cell::RefCell;
use std::rc::Rc;

use crate::rom::{MapperName, NametableArrangement, NesRom};
use crate::mappers::mapper_0::Mapper0;

pub trait Mapper {
    // Read a byte from PRG-ROM using the correctly configured bank. This address will be a value
    // a value in the range [0x8000, 0xFFFF] and the mapper must correctly take this untranslated
    // address to map it into its space. We retain the original address because some mappers will
    // leverage this as part of their bank switching.
    fn read_prg_rom_byte(&self, address: u16) -> u8;

    // Read a byte from CHR ROM using the currently configured configured bank. The address is
    // assumed to be in [0x0000, 0x1FFF].
    fn read_chr_rom_byte(&self, address: u16) -> u8;

    // Writes a byte to PRG ROM which doesn't actually write to the value in memory (because it's
    // read-only memory), but can switch banks or alter mapper state appropriately.
    fn write_prg_rom_byte(&mut self, address: u16, value: u8);

    // Write a byte into CHR ROM using the currently configured configured bank. The address is
    // assumed to be in [0x0000, 0x1FFF].
    fn write_chr_rom_byte(&mut self, address: u16, value: u8);

    // Returns the current nametable arrangement. This is static for most games, but technically
    // the mapper can change this in some titles (e.g. Zelda).
    fn get_nametable_arrangement(&self) -> NametableArrangement;
}

pub fn get_mapper(rom: &NesRom) -> Rc<RefCell<dyn Mapper>> {
    match rom.mapper {
        MapperName::Nrom => Rc::new(RefCell::new(Mapper0::from(rom))),
        _ => todo!("Unimplemented mapper {:?}", rom.mapper),
    }
}
