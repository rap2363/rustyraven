use std::{fmt, fs};
use std::fs::read_to_string;
mod addressing_modes;
mod cpu;
mod memory;
mod processor_status;

#[derive(Debug)]
enum Mapper {
    Unknown(u8),
    Nrom,
}

#[derive(Debug)]
enum NametableArrangement {
    VerticallyMirrored,
    HorizontallyMirrored,
}

struct NesRom {
    prg_rom_size: usize,
    chr_rom_size: usize,
    name_table_arrangement: NametableArrangement,
    alternative_name_table_arrangement: NametableArrangement,
    battery_backed_prg_ram: bool,
    trainer_data_present: bool,
    mapper: Mapper,
    prg_rom_data: Vec<u8>,
    chr_rom_data: Vec<u8>,
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

// Sample:
// C000  4C F5 C5  JMP $C5F5  A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7
#[derive(Debug, PartialEq)]
struct LogLine {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    sp: u8,
}

impl LogLine {
    fn from(line: &str) -> Self {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let (mut a, mut x, mut y, mut p, mut sp) = (None, None, None, None, None);
        let pc = u16::from_str_radix(tokens[0], 16).expect("First token must be parseable as an address");
        for token in &tokens[1..] {
            if token.starts_with("A:") {
                a = Some(u8::from_str_radix(&token[2..], 16).expect("A register must be parseable as a u8"));
            }
            if token.starts_with("X:") {
                x = Some(u8::from_str_radix(&token[2..], 16).expect("X register must be parseable as a u8"));
            }
            if token.starts_with("Y:") {
                y = Some(u8::from_str_radix(&token[2..], 16).expect("Y register must be parseable as a u8"));
            }
            if token.starts_with("P:") {
                p = Some(u8::from_str_radix(&token[2..], 16).expect("P register must be parseable as a u8"));
            }
            if token.starts_with("SP:") {
                sp = Some(u8::from_str_radix(&token[3..], 16).expect("SP must be parseable as a u8"));
            }
        }

        let (a, x, y, p, sp) = (a.unwrap(), x.unwrap(), y.unwrap(), p.unwrap(), sp.unwrap());
        Self { pc, a, x, y, p, sp }
    }

    pub fn from_cpu(cpu: &cpu::Cpu) -> Self {
        Self {pc: cpu.pc, a: cpu.a, x: cpu.x, y: cpu.y, p: cpu.processor_status.into(), sp: cpu.sp }
    }

    pub fn to_string(&self) -> String {
        format!("PC:0x{:04X}, A:0x{:02X}, X:0x{:02X}, Y:0x{:02X}, P:0x{:02X}, SP:0x{:02X}", self.pc, self.a, self.x, self.y, self.p, self.sp)
    }
}

#[derive(Debug)]
struct NesLog {
    pub lines: Vec<(String, LogLine)>,
}

impl NesLog {
    pub fn from_file_path(filepath: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut lines = Vec::new();

        for line in read_to_string(filepath).unwrap().lines() {
            lines.push((line.to_string(), LogLine::from(line)));
        }

        Ok(Self { lines })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Official Opcodes");
    let nes_rom = NesRom::from_file_path("src/resources/nestest.nes")?;
    let nes_log = NesLog::from_file_path("src/resources/nestest.log")?;

    let mut cpu = cpu::Cpu::initialize();
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    cpu.pc = 0xC000;

    // Now we will loop and test instructions, ensuring our log is matched each time.
    let mut i = 0;
    loop {
        let expected_log_line = &nes_log.lines[i];
        println!("{}", expected_log_line.0);
        let current_log_line = LogLine::from_cpu(&cpu);
        if expected_log_line.1 != current_log_line {
            println!("Mismatch at line {}", i + 1);
            println!("Exp: {}", expected_log_line.1.to_string());
            println!("You: {}", current_log_line.to_string());
            break;
        }
        i += 1;
        cpu.fetch_instruction_and_execute();
    }

    Ok(())
}