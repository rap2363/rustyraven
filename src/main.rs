mod addressing_modes;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes")?;

    let mut cpu = cpu::Cpu::initialize();
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    cpu.ppu().borrow_mut().write_chr_rom_data(&nes_rom.chr_rom_data);

    println!("NMI Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFA));
    println!("RES Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFC));
    println!("IRQ Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFE));

    // Read from a RESET interrupt
    cpu.pc = cpu.memory.read_two_bytes(0xFFFC);
    cpu.cycle_count = 7;

    let mut i = 0;
    // let mut vblank_latch = true;
    // let mut prev_nmi_line = false;
    // let mut seen_data = HashSet::new();

    loop {
        // Execute one cycle for the CPU
        let _b = cpu.execute_cycles_for_one_instruction();
        // Execute 3 cycles for the ppu.
        let ppu = cpu.ppu();
        ppu.borrow_mut().execute_cycle();
        ppu.borrow_mut().execute_cycle();
        ppu.borrow_mut().execute_cycle();

        // Check for an NMI and set the interrupt.
        // This is still a *little* hacky because the read from 2002 clears the Vblank flag on that register.
        if cpu.ppu().borrow().nmi() && cpu.ppu().borrow_mut().read_io_register(0x2002) & 0x80 == 0x80 {
            cpu.set_nmi();
        }

        // Checks NMI and vblank inside the PPU.
        // let nmi_line = ppu.borrow().nmi();
        // if nmi_line && !prev_nmi_line {
        //     cpu.set_nmi();
        // }
        // prev_nmi_line = nmi_line;

        // Check for a vblank and set the interrupt.
        // if cpu.ppu().borrow().vblank() && vblank_latch {
        //     cpu.set_nmi();
        //     vblank_latch = false;
        // }

        // if !cpu.ppu().borrow().vblank() && !vblank_latch {
        //     vblank_latch = true;
        // }

        // let data = cpu.memory.read_byte(0x2007);
        // if !seen_data.contains(&data) {
        //     println!("0x{:04X}=0x{:02X}", 0x2007, data);
        //     seen_data.insert(data);
        // }

        // println!("{:?}", cpu.to_string());

        // if !ppu.borrow().status().is_vblank() && vblank_latch {
        //     // Turn the latch back off when we clear the vblank.
        //     vblank_latch = false;
        // }

        i += 1;
        // for addr in 0x2000..=0x2007 {
        //     println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
        // }
        // let addr = 0x4014;
        // println!("0x{:04X}=0x{:02X}", addr, cpu.memory.read_byte(addr));
        // if cpu.memory.read_byte(0x2006) != 0x00 {
        //     println!("0x{:04X}=0x{:02X}", 0x2006, cpu.memory.read_byte(0x2006));
        // }
        // if cpu.memory.read_byte(0x2007) != 0x00 && cpu.memory.read_byte(0x2007) != 0x24 {
        //     println!("0x{:04X}=0x{:02X}", 0x2007, cpu.memory.read_byte(0x2007));
        // }
    }

    Ok(())
}