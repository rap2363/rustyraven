mod addressing_modes;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

// Rendering code, consider moving
// TODO: Move this code once you confirm it's WAI
const L: usize = 256;
const H: usize = 240;

use eframe::egui;
use egui::{ColorImage};
use std::sync::mpsc;

struct Emulation {
    texture: Option<egui::TextureHandle>,
    rx: mpsc::Receiver<egui::ColorImage>, // Channel to receive images we'll display
}

impl eframe::App for Emulation {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
         // Drain the channel; keep only the newest image if several queued up.
        if let Some(image) = self.rx.try_iter().last() {
            match &mut self.texture {
                // Texture exists: update its pixels in place on the GPU.
                Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
                // First image ever: create the texture.
                None => {
                    self.texture = Some(ui.ctx().load_texture(
                        "emulation_image",
                        image,
                        egui::TextureOptions::NEAREST,
                    ))
                }
            }
        }

         // Scale the image to fill the window, recomputed every frame.
        match &self.texture {
            Some(texture) => {
                ui.centered_and_justified(|ui| {
                    ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(ui.available_size()),
                    );
                });
            }
            None => {
                ui.label("waiting for first frame...");
            }
        }
        // egui::Frame::central_panel(ui.style()).show(ui, |ui| {
        //     match &self.texture {
        //         Some(texture) => { ui.image(texture); }
        //         None => { ui.label("waiting for first frame..."); }
        //     }
        // });
    }
}

// TODO (replace this with whatever)
fn produce_images(tx: mpsc::Sender<egui::ColorImage>, ctx: egui::Context) {
    // Initializing Code for CPU
    let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes").expect("File not found!");

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

    loop {
        if let Some(image) = main_nes_loop(&mut cpu) && tx.send(image).is_err() {
            return; // window closed, receiver dropped
        }

        // if tx.send(egui::ColorImage { size: [L, H], source_size: Vec2::new(L as f32, H as f32), pixels }).is_err() {
        //     return; // window closed, receiver dropped
        // }
        ctx.request_repaint(); // wake the UI so it actually draws the new frame
     }
}

fn main_nes_loop(cpu: &mut cpu::Cpu) -> Option<ColorImage> {
    // Execute one cycle for the CPU
    cpu.execute_cycles_for_one_instruction();
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

    if let Some(pixels) = cpu.ppu().borrow().get_image() {
        // Ignore if pixel length not corrrect
        if pixels.len() != 256 * 240 {
            return None;
        }

        std::thread::sleep(std::time::Duration::from_millis(14)); // ~60 fps

        let mut color_image_pixels = Vec::with_capacity(256 * 240);
        // Otherwise we'll convert our RGB pixels.
        for ppu::Pixel(r, g, b) in pixels.into_iter() {
            color_image_pixels.push(egui::Color32::from_rgb(r, g, b));
        }
        return Some(egui::ColorImage {
            size: [L, H],
            source_size: egui::Vec2::new(L as f32, H as f32),
            pixels: color_image_pixels,
        });
    }
    None
}

fn main() -> eframe::Result<()> {
    // // Initializing Code for CPU
    // let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes").expect("File not found!");

    // let mut cpu = cpu::Cpu::initialize();
    // // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    // cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // // NROM means we write it to the lower and upper banks.
    // cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    // cpu.ppu().borrow_mut().write_chr_rom_data(&nes_rom.chr_rom_data);

    // println!("NMI Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFA));
    // println!("RES Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFC));
    // println!("IRQ Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFE));

    // // Read from a RESET interrupt
    // cpu.pc = cpu.memory.read_two_bytes(0xFFFC);
    // cpu.cycle_count = 7;

    let (tx, rx) = mpsc::channel();

    // Make the window exactly the size of the image.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([L as f32, H as f32])
            .with_resizable(true),
        ..Default::default()
    };
 
    eframe::run_native(
        "Rusty Raven",
        options,
        Box::new(move |cc| {
            // Clone the Context here so the producer can request repaints.
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || produce_images(tx, ctx));
            Ok(Box::new(Emulation { rx, texture: None }))
        }),
    )

    // loop {
    //     // Execute one cycle for the CPU
    //     cpu.execute_cycles_for_one_instruction();
    //     // Execute 3 cycles for the ppu.
    //     let ppu = cpu.ppu();
    //     ppu.borrow_mut().execute_cycle();
    //     ppu.borrow_mut().execute_cycle();
    //     ppu.borrow_mut().execute_cycle();

    //     // Check for an NMI and set the interrupt.
    //     // This is still a *little* hacky because the read from 2002 clears the Vblank flag on that register.
    //     if cpu.ppu().borrow().nmi() && cpu.ppu().borrow_mut().read_io_register(0x2002) & 0x80 == 0x80 {
    //         cpu.set_nmi();
    //     }
    // }
}