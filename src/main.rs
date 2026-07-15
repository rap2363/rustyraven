use std::{fmt, fs};
mod addressing_modes;
mod cpu;
mod memory;
mod processor_status;
mod rom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes")?;

    let mut cpu = cpu::Cpu::initialize();
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);

    println!("NMI Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFA));
    println!("RES Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFC));
    println!("IRQ Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFE));

    // Read from a RESET interrupt
    cpu.pc = cpu.memory.read_two_bytes(0xFFFC);
    cpu.cycle_count = 7;

    let mut i = 0;
    while i <= 1000 {
        if i == 10 {
            println!("Interrupt!");
            cpu.set_nmi();
        }
        let _b = cpu.execute_cycles_for_one_instruction();
        if !_b {
            continue;
        }
        println!("{}", cpu.to_string());
        i += 1;
        // cpu.fetch_instruction_and_execute();
        // for addr in 0x2000..=0x2005 {
        //     println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
        // }
        // let addr = 0x4014;
        // println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
    }

    Ok(())
}