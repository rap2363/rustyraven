use std::{fmt, fs};
use std::fs::read_to_string;
mod addressing_modes;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

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
    cyc: usize,
}

impl LogLine {
    fn from(line: &str) -> Self {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let (mut a, mut x, mut y, mut p, mut sp, mut cyc) = (None, None, None, None, None, None);
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
            if token.starts_with("CYC:") {
                cyc = Some(usize::from_str_radix(&token[4..], 10).expect("CYC must be parseble as a usize"));
            }
        }

        let (a, x, y, p, sp, cyc) = (a.unwrap(), x.unwrap(), y.unwrap(), p.unwrap(), sp.unwrap(), cyc.unwrap());
        Self { pc, a, x, y, p, sp, cyc}
    }

    pub fn from_cpu(cpu: &cpu::Cpu) -> Self {
        Self {pc: cpu.pc, a: cpu.a, x: cpu.x, y: cpu.y, p: cpu.processor_status.into(), sp: cpu.sp, cyc: cpu.cycle_count }
    }

    pub fn to_string(&self) -> String {
        format!("PC:0x{:04X}, A:0x{:02X}, X:0x{:02X}, Y:0x{:02X}, P:0x{:02X}, SP:0x{:02X}, CYC:{}", self.pc, self.a, self.x, self.y, self.p, self.sp, self.cyc)
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
    let nes_rom = rom::NesRom::from_file_path("src/resources/nestest.nes")?;
    let nes_log = NesLog::from_file_path("src/resources/nestest.log")?;

    let mut cpu = cpu::Cpu::initialize();
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    cpu.pc = 0xC000;
    cpu.cycle_count = 7;

    // Now we will loop and test instructions, ensuring our log is matched each time.
    let mut i = 0;
    loop {
        let expected_log_line = &nes_log.lines[i];
        println!("{i}: {}", expected_log_line.0);
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