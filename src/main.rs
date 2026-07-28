mod addressing_modes;
mod controller;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

const DMA_CPU_CYCLE_COUNT: i32 = 512;
const NUM_FRAME_CYCLES: usize = 87296; // 256 * 341

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
    }
}

// TODO (replace this with whatever)
fn produce_images(tx: mpsc::Sender<egui::ColorImage>, ctx: egui::Context) {
    // Initializing Code for CPU
    let nes_rom = rom::NesRom::from_file_path("src/resources/smb.nes").expect("File not found!");

    let mut cpu = cpu::Cpu::initialize();
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    if nes_rom.prg_rom_data.len() == 0x4000 {
        // Copy it to the upper bank as well.
        cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    }
    cpu.bus.borrow_mut().ppu().write_chr_rom_data(&nes_rom.chr_rom_data);

    println!("NMI Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFA));
    println!("RES Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFC));
    println!("IRQ Address: 0x{:4X}", cpu.memory.read_two_bytes(0xFFFE));

    // Read from a RESET interrupt
    cpu.pc = cpu.memory.read_two_bytes(0xFFFC);
    cpu.cycle_count = 7;

    loop {
        for (key, controller_button) in vec![
            (egui::Key::A, controller::Button::A),
            (egui::Key::S, controller::Button::B),
            (egui::Key::ShiftRight, controller::Button::Select),
            (egui::Key::Enter, controller::Button::Start),
            (egui::Key::ArrowLeft, controller::Button::Left),
            (egui::Key::ArrowRight, controller::Button::Right),
            (egui::Key::ArrowUp, controller::Button::Up),
            (egui::Key::ArrowDown, controller::Button::Down),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                cpu.bus.borrow_mut().controller_1().set_button(controller_button);
            }

            if ctx.input(|i| i.key_released(key)) {
                cpu.bus.borrow_mut().controller_1().clear_button(controller_button);
            }
        }

        if let Some(image) = main_nes_loop(&mut cpu) && tx.send(image).is_err() {
            return; // window closed, receiver dropped
        }

        ctx.request_repaint(); // wake the UI so it actually draws the new frame
     }
}

fn main_nes_loop(cpu: &mut cpu::Cpu) -> Option<ColorImage> {
    // Execute one cycle for the CPU
    cpu.execute_cycles_for_one_instruction();
    // Execute 3 cycles for the ppu.
    cpu.bus.borrow_mut().ppu().execute_cycle();
    cpu.bus.borrow_mut().ppu().execute_cycle();
    cpu.bus.borrow_mut().ppu().execute_cycle();

    // Check for an NMI and set the interrupt.
    // This is still a *little* hacky because the read from 2002 clears the Vblank flag on that register.
    if cpu.bus.borrow_mut().ppu().nmi() && cpu.bus.borrow_mut().ppu().read_io_register(0x2002) & 0x80 == 0x80 {
        cpu.set_nmi();
    }

    // Check for a DMA and stall the cpu for a number of cycles if it did.
    if cpu.bus.borrow_mut().ppu().dma_flag() {
        cpu.cycle_budget -= DMA_CPU_CYCLE_COUNT;
    }

    if let Some(pixels) = cpu.bus.borrow_mut().ppu().get_image() {
        // TODO: We should do this on a fixed interval runtime but threading in tokio is going
        // to be a hassle..
        std::thread::sleep(std::time::Duration::from_millis(11));

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
}