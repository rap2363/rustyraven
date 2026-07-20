use std::{fmt, fs};
mod addressing_modes;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes")?;

    let mut cpu = cpu::Cpu::initialize();
    let mut ppu = ppu::Ppu::initialize();
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
    let mut vblank_latch = false;
    while i <= 1000000 {
        // Execute one cycle for the CPU
        let _b = cpu.execute_cycles_for_one_instruction();
        // Execute 3 cycles for the ppu.
        ppu.execute_cycle();
        ppu.execute_cycle();
        ppu.execute_cycle();

        // Copy over PPU registers.
        cpu.memory.write_byte(0x2000, ppu.control().into());
        cpu.memory.write_byte(0x2002, ppu.status().into());
        if ppu.status().is_vblank() && !vblank_latch {
            // Triggers the very first time we set the vblank, but not after that.
            cpu.set_nmi();
            vblank_latch = true;
        }

        if !ppu.status().is_vblank() && vblank_latch {
            // Turn the latch back off when we clear the vblank.
            vblank_latch = false;
        }

        i += 1;
        // for addr in 0x2000..=0x2007 {
        //     println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
        // }
        // let addr = 0x4014;
        // println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
        // if cpu.memory.read_byte(0x2006) != 0x00 {
        //     println!("0x{:04X}=0x{:02X}", 0x2006, cpu.memory.read_byte(0x2006));
        // }
        // if cpu.memory.read_byte(0x2007) != 0x00 {
        //     println!("0x{:04X}=0x{:02X}", 0x2007, cpu.memory.read_byte(0x2007));
        // }
    }

    Ok(())
}