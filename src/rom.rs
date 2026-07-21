use std::{fmt, fs};

#[derive(Debug)]
pub enum Mapper {
    Unknown(u8),
    Nrom,
}

#[derive(Debug)]
pub enum NametableArrangement {
    VerticallyMirrored,
    HorizontallyMirrored,
}

pub struct NesRom {
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub name_table_arrangement: NametableArrangement,
    pub alternative_name_table_arrangement: NametableArrangement,
    pub battery_backed_prg_ram: bool,
    pub trainer_data_present: bool,
    pub mapper: Mapper,
    pub prg_rom_data: Vec<u8>,
    pub chr_rom_data: Vec<u8>,
}

// Manually implement Debug for NesRom
impl fmt::Debug for NesRom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NesRom")
            .field("prg_rom_size", &self.prg_rom_size)
            .field("chr_rom_size", &self.chr_rom_size)
            .field("name_table_arrangement", &self.name_table_arrangement)
            .field("alternative_name_table_arrangement", &self.alternative_name_table_arrangement)
            .field("battery_backed_prg_ram", &self.battery_backed_prg_ram)
            .field("trainer_data_present", &self.trainer_data_present)
            .finish()
    }
}

impl NesRom {
    pub fn from_file_path(filepath: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let nes_rom_data = fs::read(filepath)?;
        // First 16 bytes correspond to various header data and parameters.
        // The first four characters match to "NES" in ASCII followed by the MS-DOS EOF marker.
        if nes_rom_data[..=3] != [0x4E, 0x45, 0x53, 0x1A] {
            return Err("Not a valid .NES file, first four characters do not equal proper sequence.".into());
        }

        let prg_rom_size = 16384 * (nes_rom_data[4] as usize); // 16 kb units
        let chr_rom_size = 8192 * (nes_rom_data[5] as usize); // 8 kb units
        let flags_six = nes_rom_data[6];
        let flags_seven = nes_rom_data[7];
        let name_table_arrangement = if (flags_six & 0x01) == 0x01 {
            NametableArrangement::HorizontallyMirrored
        } else {
            NametableArrangement::VerticallyMirrored
        };
        let alternative_name_table_arrangement = if ((flags_six >> 3) & 0x01) == 0x01 {
            NametableArrangement::HorizontallyMirrored
        } else {
            NametableArrangement::VerticallyMirrored
        };
        let battery_backed_prg_ram = (flags_six >> 1) & 0x01 == 0x01;
        let trainer_data_present = (flags_six >> 2) & 0x01 == 0x01;
        let mapper_value = (flags_seven & 0xF0) + (flags_six >> 4);
        let mapper = match mapper_value {
            0x00 => Mapper::Nrom,
            x => Mapper::Unknown(x),
        };
        let trainer_num_bytes = if trainer_data_present { 512 } else { 0 };
        let prg_rom_offset = 16 + trainer_num_bytes;
        let chr_rom_offset = prg_rom_offset + prg_rom_size;
        let prg_rom_data = nes_rom_data[prg_rom_offset..(prg_rom_offset + prg_rom_size)].to_vec();
        let chr_rom_data = nes_rom_data[chr_rom_offset..(chr_rom_offset + chr_rom_size)].to_vec();
        Ok(Self {
            prg_rom_size,
            chr_rom_size,
            name_table_arrangement,
            alternative_name_table_arrangement,
            battery_backed_prg_ram,
            trainer_data_present,
            mapper,
            prg_rom_data,
            chr_rom_data
        })
    }
}
