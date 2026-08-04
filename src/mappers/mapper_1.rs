use crate::memory::Segment;
use crate::mappers::mapper::Mapper;
use crate::rom::{NametableArrangement, NesRom};

// Mapper 1 (MMC-1) has a slightly more complicated scheme than mapper 2. Specifically, it uses writes
// to the 0x8000 registers to serialize in a "register" load/target value, and then performs some
// appropriate configuration based on that.
// Link: https://www.nesdev.org/wiki/MMC1
//
// Specifically, ROM capacity is 256kb (512 kb), allowing us to index 32 16kb banks to swap. Our PRG-ROM 
// window can *either* be a dynamic 16kb window plus a fixed 16kb window *or* a single 32 kb window. 
// Similarly our CHR-ROM has 128 kb available, with 4kb + 4kb fixed or 8kb windowing, for 32 banks.
// This mapper also includes PRG-RAM support, with an 8kb window in [0x6000, 0x8000] (the SRAM). We
// have 32 kb capacity for RAM, giving us 4 banks to switch between.
//
// Register Writes ($8000-$FFFF)
// Writes are 1 bit at a time serially to create a 5 bit value.
// The *zone* of memory we write into determines the kind of operation we make with the 5 bits.
// There are four zones total:
// [0x8000, 0x9FFF]: Control
// [0xA000, 0xBFFF]: CHR Bank 0
// [0xC000, 0xDFFF]: CHR Bank 1
// [0xE000, 0xFFFF]: PRG Bank
//
// Load register ($8000-$FFFF)
// 7  bit  0
// ---- ----
// Rxxx xxxD
// |       |
// |       +- Data bit to be shifted into shift register, LSB first
// +--------- A write with bit set will reset shift register
//             and write Control with (Control OR $0C), 
//             locking PRG-ROM at $C000-$FFFF to the last bank.
// 
// Control (internal, $8000-$9FFF)
// 
// 4bit0
// -----
// CPPMM
// |||||
// |||++- Nametable arrangement: (0: one-screen, lower bank; 1: one-screen, upper bank;
// |||               2: horizontal arrangement ("vertical mirroring", PPU A10); 
// |||               3: vertical arrangement ("horizontal mirroring", PPU A11) )
// |++--- PRG-ROM bank mode (0, 1: switch 32 KB at $8000, ignoring low bit of bank number;
// |                         2: fix first bank at $8000 and switch 16 KB bank at $C000;
// |                         3: fix last bank at $C000 and switch 16 KB bank at $8000)
// +----- CHR-ROM bank mode (0: switch 8 KB at a time; 1: switch two separate 4 KB banks)
// 
// CHR bank 0 (internal, $A000-$BFFF)
// 
// 4bit0
// -----
// CCCCC
// |||||
// +++++- Select 4 KB or 8 KB CHR bank at PPU $0000 (low bit ignored in 8 KB mode)
//
// CHR bank 1 (internal, $C000-$DFFF)
// 
// 4bit0
// -----
// CCCCC
// |||||
// +++++- Select 4 KB CHR bank at PPU $1000 (ignored in 8 KB mode)
//
// PRG bank (internal, $E000-$FFFF)
//
// 4bit0
// -----
// RPPPP
// |||||
// |++++- Select 16 KB PRG-ROM bank (low bit ignored in 32 KB mode)
// +----- MMC1A:
//        0: fixed bank affects A17..A14
//        1: fixed bank only affects A16..A14, bit 3 directly controls A17 across the entire $8000-$FFFF address range
//        MMC1B:
//        0: PRG-RAM enabled
//        1: PRG-RAM disabled
//
// So for example, if we did 5 writes 0x01, 0x00, 0x01, 0x01, 0x00 with the last at 0x8001, this 
// would code to a kind of abstract: CONTROL = 01101 --> (8kb mode, fix last bank at 0xC000 and switch 16 kb at 0x8000, use one-screen)
//
// Schematic: 
// Because of how complex the mapping is, it doesn't really make sense to try and label the zones according to the bank number. We'll
// simply divvy up the zones to illustrate particular regions.
// 
// CPU:
//
// 0x6000_____________
// |  PRG-RAM (8 kb)  |
// |                  |
// 0x8000______________
// |     Control      |
// |                  |
// 0xA000_____________|
// |     CHR-Bank 0   |
// |                  |
// 0xC000_____________|
// |     CHR-Bank 1   |
// |                  |
// 0xE000_____________|
// |     PRG Bank     |
// |                  |
// 0x10000____________|
//
// PPU
//
// 0x0000______________
// |    CHR-ROM/RAM   |
// |       (8 kb)     |
// 0x2000_____________|
//
// This mapper will need to hold a lot of specific state and allow for an internal "reset" to take place.

// PRG-ROM bank mode 
// 
// 0, 1: switch 32 KB at $8000, ignoring low bit of bank number
//    2: fix first bank at $8000 and switch 16 KB bank at $C000
//    3: fix last bank at $C000 and switch 16 KB bank at $8000
#[derive(Clone, Copy, Debug, PartialEq)]
enum PrgBankMode {
    Switch32Kb,
    FixFirstBank,
    FixLastBank,
}

// CHR-ROM bank mode enums.
// 0: switch 8 KB at a time
// 1: switch two separate 4 KB banks
#[derive(Clone, Copy, Debug, PartialEq)]
enum ChrBankMode {
    Switch8Kb,
    Switch4Kb,
}

enum Register {
    Control,
    ChrBank0,
    ChrBank1,
    PrgBank,
}

#[derive(Clone, Copy)]
struct ControlState {
    current_nt: NametableArrangement,
    prg_bank_mode: PrgBankMode,
    chr_bank_mode: ChrBankMode,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    bit_count: u8,
    data: u8,
}

impl ControlState {

    fn initialize() -> Self {
        Self {
            current_nt: NametableArrangement::SingleScreenLo,
            prg_bank_mode: PrgBankMode::FixLastBank,
            chr_bank_mode: ChrBankMode::Switch8Kb,
            chr_bank_0: 0x00,
            chr_bank_1: 0x00,
            prg_bank: 0x00,
            bit_count: 0x00, // Bit count stores a value from 0 -> 4 and gets reset after shifting bits.
            data: 0x00, // This data is stored as state between writes.
        }
    }

    // To reset the mapper, which clears the shift register and sets the PRG-ROM bank mode to 3 
    // (fixing the last bank at $C000 and allowing the 16 KB bank at $8000 to be switched), one 
    // need only do a single write to any ROM address with a 1 in bit 7:
    //
    // resetMapper:
    //   lda #$80
    //   sta $8000
    //   rts
    //
    // The CHR-ROM bank mode and nametable arrangement are not altered.
    fn reset(&mut self) {
        self.prg_bank_mode = PrgBankMode::FixLastBank;
        self.bit_count = 0;
        self.data = 0x00;
    }
}

pub struct Mapper1 {
    // prg_rom: Segment<0x80000>, // 512 kb
    prg_rom: Segment<0x20000>, // 128 kb
    num_prg_rom_banks: usize,
    prg_ram: Segment<0x2000>, // 8 kb
    chr_rom: Segment<0x8000>, // 32 kb
    control_state: ControlState,
}

impl Mapper1 {
    pub fn from(rom: &NesRom) -> Self {
        // let mut prg_rom = Segment::<0x80000>::initialize();
        let mut prg_rom = Segment::<0x20000>::initialize();
        let prg_ram = Segment::<0x2000>::initialize();
        let mut chr_rom = Segment::<0x8000>::initialize();
        let control_state = ControlState::initialize();

        // Now if this exceeds our capacity that's some bad data.
        prg_rom.write_bytes(0x00, &rom.prg_rom_data);
        let num_prg_rom_banks = &rom.prg_rom_size / 0x4000;

        // This might be zero. If it is that's actually totally fine (it's RAM).
        chr_rom.write_bytes(0x00, &rom.chr_rom_data);

        Self { prg_rom, num_prg_rom_banks, prg_ram, chr_rom, control_state }
    }

    fn test_initialize() -> Self {
        // let prg_rom = Segment::<0x80000>::initialize();
        let prg_rom = Segment::<0x20000>::initialize();
        let num_prg_rom_banks = 8;
        let prg_ram = Segment::<0x2000>::initialize();
        let chr_rom = Segment::<0x8000>::initialize();
        let control_state = ControlState::initialize();

        Self { prg_rom, num_prg_rom_banks, prg_ram, chr_rom, control_state }
    }

    // Control
    // 
    // 4bit0
    // -----
    // CPPMM
    // |||||
    // |||++- Nametable arrangement: (0: one-screen, lower bank; 1: one-screen, upper bank;
    // |||               2: horizontal arrangement ("vertical mirroring", PPU A10); 
    // |||               3: vertical arrangement ("horizontal mirroring", PPU A11) )
    // |++--- PRG-ROM bank mode (0, 1: switch 32 KB at $8000, ignoring low bit of bank number;
    // |                         2: fix first bank at $8000 and switch 16 KB bank at $C000;
    // |                         3: fix last bank at $C000 and switch 16 KB bank at $8000)
    // +----- CHR-ROM bank mode (0: switch 8 KB at a time; 1: switch two separate 4 KB banks)
    fn write_control(&mut self, value: u8) {
        self.control_state.current_nt = match value & 0x03 {
            0x00 => NametableArrangement::SingleScreenLo,
            0x01 => NametableArrangement::SingleScreenHi,
            0x02 => NametableArrangement::VerticallyMirrored,
            0x03 => NametableArrangement::HorizontallyMirrored,
            _ => panic!("Invalid state: {}", value),
        };

        self.control_state.prg_bank_mode = match (value >> 2) & 0x03 {
            0x00 | 0x01 => PrgBankMode::Switch32Kb,
            0x02 => PrgBankMode::FixFirstBank,
            0x03 => PrgBankMode::FixLastBank,
            _ => panic!("Invalid state: {}", value),
        };
        self.control_state.chr_bank_mode = match (value >> 4) & 0x01 {
            0x00 => ChrBankMode::Switch8Kb,
            0x01 => ChrBankMode::Switch4Kb,
            _ => panic!("Invalid state: {}", value),
        };
    }

    // CHR bank 0 (internal, $A000-$BFFF)
    //
    // 4bit0
    // -----
    // CCCCC
    // |||||
    // +++++- Select 4 KB or 8 KB CHR bank at PPU $0000 (low bit ignored in 8 KB mode)
    fn write_chr_bank_0(&mut self, value: u8) {
        self.control_state.chr_bank_0 = value;
    }

    // CHR bank 1 (internal, $C000-$DFFF)
    //
    // 4bit0
    // -----
    // CCCCC
    // |||||
    // +++++- Select 4 KB CHR bank at PPU $1000 (ignored in 8 KB mode)
    fn write_chr_bank_1(&mut self, value: u8) {
        self.control_state.chr_bank_1 = value;
    }

    // PRG bank (internal, $E000-$FFFF)
    //
    // 4bit0
    // -----
    // RPPPP
    // |||||
    // |++++- Select 16 KB PRG-ROM bank (low bit ignored in 32 KB mode)
    // +----- MMC1A:
    //        0: fixed bank affects A17..A14
    //        1: fixed bank only affects A16..A14, bit 3 directly controls A17 across the entire $8000-$FFFF address range
    //        MMC1B:
    //        0: PRG-RAM enabled
    //        1: PRG-RAM disabled
    fn write_prg_bank(&mut self, value: u8) {
        self.control_state.prg_bank = value & 0x0F;
        if value & 0x10 == 0x10 {
            panic!("Unimplemented write for PRG Bank");
        }
    }

    // Assumed this address is in [0x8000, 0xFFFF], we reference either *one*
    // 32 kb bank we've mapped via control_state.prg_bank or two smaller 8 kb banks
    // using the fixed form mapping. Very similar to the chr_rom addressing, but using
    // fixed banks.
    fn get_mapped_prg_address(&self, address: u16) -> usize {
        let mut offset_address = (address - 0x8000) as usize;
        let base_address = match self.control_state.prg_bank_mode {
            PrgBankMode::Switch32Kb => ((self.control_state.prg_bank & 0x0E) as usize) * 0x4000,
            PrgBankMode::FixFirstBank => {
                if offset_address < 0x4000 {
                    // Use the first bank.
                    0x0000
                } else {
                    // Pick the 4 kb bank based for the latter half.
                    offset_address -= 0x4000;
                    (self.control_state.prg_bank & 0x07) as usize * 0x4000
                }
            },
            PrgBankMode::FixLastBank => {
                if offset_address < 0x4000 {
                    // Pick the 4kb bank.
                    (self.control_state.prg_bank & 0x07) as usize * 0x4000
                } else {
                    offset_address -= 0x4000;
                    (self.num_prg_rom_banks - 1) * 0x4000
                }
            },
        };

        base_address + offset_address
    }

    // Assumed this address < 0x2000 (as it's CHR ROM), so we either write to chr_bank_0 or
    // chr_bank_1 based on the 4 or 8kb switching mode.
    // For 8kb, the chr_bank_0 indexes into 8kb pages into our CHR ROM. but we ignore the lower
    // bit, so 00101 would translate to page 4 (not 5).
    // For 4 kb we either write to chr_bank_0 or chr_bank_1 depending on the address (if it's
    // < 0x1000 then we write to chr_bank_0 and chr_bank_1 otherwise).
    fn get_mapped_chr_address(&self, address: u16) -> usize {
        let mut offset_address = address as usize;
        let base_address = match self.control_state.chr_bank_mode {
            ChrBankMode::Switch8Kb => ((self.control_state.chr_bank_0 & 0x0E) as usize) * 0x1000,
            ChrBankMode::Switch4Kb => {
                if offset_address < 0x1000 {
                    (self.control_state.chr_bank_0 & 0x07) as usize * 0x1000
                } else {
                    offset_address -= 0x1000;
                    (self.control_state.chr_bank_1 & 0x07) as usize * 0x1000
                }
            },
        };

        base_address + offset_address
    }
}

impl Mapper for Mapper1 {
    // Read a byte from PRG-RAM. There's only one 8 kb bank currently mapped to this space, so the 
    // address is assumed to be in [0x6000, 0x7FFF].
    fn read_prg_ram_byte(&self, address: u16) -> u8 {
        let ram_address = (address - 0x6000) as usize;
        self.prg_ram.read_byte(ram_address)
    }

    fn read_prg_rom_byte(&self, address: u16) -> u8 {
        self.prg_rom.read_byte(self.get_mapped_prg_address(address))
    }

    fn read_chr_rom_byte(&self, address: u16) -> u8 {
        self.chr_rom.read_byte(self.get_mapped_chr_address(address))
    }

    // This is part of a series of writes. We either:
    // 1. shift in bit 0 of the write into our data register.
    // 2. Reset if bit 7 of the write is set.
    fn write_prg_rom_byte(&mut self, address: u16, value: u8) {
        if value & 0x80 == 0x80 {
            self.control_state.reset();
            return;
        }

        // Shift in the bit at bit 4.
        // e.g. if data = abc00
        // and value = 0x01,
        // data = 0x10 | 0abc0 = 1abc0
        self.control_state.data =  ((value & 0x01) << 4) | (self.control_state.data >> 1);
        self.control_state.bit_count += 1;

        if self.control_state.bit_count < 5 {
            return;
        }

        // If we shifted in 5 bits, it's time to reset the data register and do something with the
        // value. We do this based on the address we wrote to and the *zone* it corresponds to.
        let data = self.control_state.data;
        self.control_state.data = 0x00;
        self.control_state.bit_count = 0;

        let register = if address < 0xA000 {
            Register::Control
        } else if address < 0xC000 {
            Register::ChrBank0
        } else if address < 0xE000 {
            Register::ChrBank1
        } else {
            Register::PrgBank
        };

        match register {
            Register::Control => self.write_control(data),
            Register::ChrBank0 => self.write_chr_bank_0(data),
            Register::ChrBank1 => self.write_chr_bank_1(data),
            Register::PrgBank => self.write_prg_bank(data),
        }
    }

    fn write_chr_rom_byte(&mut self, address: u16, value: u8) {
        self.chr_rom.write_byte(self.get_mapped_chr_address(address), value);
    }

    fn write_prg_ram_byte(&mut self, address: u16, value: u8) {
        let ram_address = (address - 0x6000) as usize;
        self.prg_ram.write_byte(ram_address, value);
    }

    fn get_nametable_arrangement(&self) -> NametableArrangement {
        self.control_state.current_nt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writes_with_target() {
        let mut mapper = Mapper1::test_initialize();
        // We'll write the five bit value b10110
        mapper.write_prg_rom_byte(0x8000, 0x00);
        mapper.write_prg_rom_byte(0x8000, 0x01);
        mapper.write_prg_rom_byte(0x8000, 0x01);
        mapper.write_prg_rom_byte(0x8000, 0x00);
        mapper.write_prg_rom_byte(0x8000, 0x01);
        // Now after this final write we should have written to the 
        // control register.
        assert_eq!(mapper.control_state.chr_bank_mode, ChrBankMode::Switch4Kb);
        assert_eq!(mapper.control_state.prg_bank_mode, PrgBankMode::Switch32Kb);
        assert_eq!(mapper.control_state.current_nt, NametableArrangement::VerticallyMirrored);
    }

    #[test]
    fn test_get_mapped_prg_address_switch_32_kb() {
        let mut mapper = Mapper1::test_initialize();
        mapper.control_state.prg_bank_mode = PrgBankMode::Switch32Kb;
        mapper.control_state.prg_bank = 0x05;
        // Now if we have 8 4kb banks, the address should simply ignore the least
        // significant bit and we'll return (in this case), the "3rd" 8 kb bank.
        assert_eq!(mapper.get_mapped_prg_address(0x8042), 65602);  // 0x02 * 0x8000 + 0x0042
        assert_eq!(mapper.get_mapped_prg_address(0xC042), 81986); // 0x02 * 0x8000 + 0x4042
    }

    #[test]
    fn test_get_mapped_prg_address_fix_first_bank() {
        let mut mapper = Mapper1::test_initialize();
        mapper.control_state.prg_bank_mode = PrgBankMode::FixFirstBank;
        mapper.control_state.prg_bank = 0x05;
        // Now if we have 8 4kb banks, the address should simply return the fifth
        // 4 kb bank.
        assert_eq!(mapper.get_mapped_prg_address(0x8042), 66);    // 0x0042 (first bank!)
        assert_eq!(mapper.get_mapped_prg_address(0xC042), 81986); // 0x05 * 0x4000 + 0x0042
    }

    #[test]
    fn test_get_mapped_prg_address_fix_last_bank() {
        let mut mapper = Mapper1::test_initialize();
        mapper.control_state.prg_bank_mode = PrgBankMode::FixLastBank;
        mapper.control_state.prg_bank = 0x05;
        // Now if we have 8 4kb banks, the address should simply return the fifth
        // 4 kb bank when we query within [0x8000, 0xC000]
        assert_eq!(mapper.get_mapped_prg_address(0x8042), 81986);  // 0x05 * 0x4000 + 0x0042
        assert_eq!(mapper.get_mapped_prg_address(0xC042), 114754); // 0x07 * 0x4000 + 0x0042 (last bank!)
    }

    #[test]
    fn test_get_mapped_chr_rom_switch_8_kb() {
        let mut mapper = Mapper1::test_initialize();
        mapper.control_state.chr_bank_mode = ChrBankMode::Switch8Kb;
        mapper.control_state.chr_bank_0 = 0x05;
        mapper.control_state.chr_bank_1 = 0xFF;
        // Now if we have 4 8kb banks, the address should simply ignore the least
        // significant bit and we'll return (in this case), the "third" 8 kb bank.
        assert_eq!(mapper.get_mapped_chr_address(0x0042), 16450);  // 2 * 0x2000 + 0x0042 (base=0x4000)
        assert_eq!(mapper.get_mapped_chr_address(0x1042), 20546);  // 2 * 0x2000 + 0x1042 (base=0x5000)
    }

    #[test]
    fn test_get_mapped_chr_rom_switch_4kb() {
        let mut mapper = Mapper1::test_initialize();
        mapper.control_state.chr_bank_mode = ChrBankMode::Switch4Kb;
        mapper.control_state.chr_bank_0 = 0x05;
        mapper.control_state.chr_bank_1 = 0xFF;
        // We have 8 4 kb banks, swap between them.
        assert_eq!(mapper.get_mapped_chr_address(0x0042), 20546);  //  0x5000 + 0x0042 (base=0x5000)
        assert_eq!(mapper.get_mapped_chr_address(0x1042), 28738);  //  0x7000 + 0x0042 (base=0x7000)
    }
}