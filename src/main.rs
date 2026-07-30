mod addressing_modes;
mod controller;
mod cpu;
mod memory;
mod ppu;
mod ppu_registers;
mod processor_status;
mod rom;

const DMA_CPU_CYCLE_COUNT: i32 = 512;
const NUM_FRAME_CYCLES: usize = 89001; // 261 * 341

// Rendering code, consider moving
// TODO: Move this code once you confirm it's WAI
const L: usize = 256;
const H: usize = 240;

use eframe::egui;
use egui::{ColorImage};
use std::{sync::mpsc, thread::sleep, time::Instant};

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
    let nes_rom = rom::NesRom::from_file_path("src/resources/donkey_kong.nes").expect("File not found!");

    let mut cpu = cpu::Cpu::initialize();
    // TODO: This is all fine for no mapper, but will break otherwise.
    // Load the prg_rom data into main memory starting at 0x8000-0xFFFF
    cpu.memory.write_bytes(0x8000, &nes_rom.prg_rom_data);
    // NROM means we write it to the lower and upper banks.
    if nes_rom.prg_rom_data.len() == 0x4000 {
        // Copy it to the upper bank as well.
        cpu.memory.write_bytes(0xC000, &nes_rom.prg_rom_data);
    }
    cpu.bus.borrow_mut().ppu().write_chr_rom_data(&nes_rom.chr_rom_data);

    // Read from a RESET interrupt
    cpu.pc = cpu.memory.read_two_bytes(0xFFFC);
    cpu.cycle_count = 7;

    const STANDARD_CYCLE_SLEEP: std::time::Duration = std::time::Duration::from_millis(9);

    let mut sleep_duration = STANDARD_CYCLE_SLEEP;
    let mut now = std::time::Instant::now();
    let mut num_frames = 0;

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

        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            sleep_duration = std::time::Duration::from_millis(2);
        }

        if ctx.input(|i| i.key_released(egui::Key::Tab)) {
            sleep_duration = STANDARD_CYCLE_SLEEP;
        }

        if let Some(image) = main_nes_loop(&mut cpu, sleep_duration) {
            if tx.send(image).is_err() {
                return; // window closed.
            }
            num_frames += 1;
            let seconds_per_frame = now.elapsed().as_micros() as f64;
            if num_frames % 100 == 0 {
                println!("FPS: {}", 1_000_000.0 / seconds_per_frame);
            }
            now = std::time::Instant::now();
        }

        ctx.request_repaint(); // wake the UI so it actually draws the new frame
     }
}

fn main_nes_loop(cpu: &mut cpu::Cpu, sleep_duration: std::time::Duration) -> Option<ColorImage> {
    // Execute one cycle for the CPU
    cpu.execute_cycles_for_one_instruction();
    // Execute 3 cycles for the ppu.
    cpu.bus.borrow_mut().ppu().execute_cycle();
    cpu.bus.borrow_mut().ppu().execute_cycle();
    cpu.bus.borrow_mut().ppu().execute_cycle();

    // Check for an NMI and set the interrupt.
    if cpu.bus.borrow_mut().ppu().nmi() {
        cpu.set_nmi();
    }

    // Check for a DMA and stall the cpu for a number of cycles if it did.
    if cpu.bus.borrow_mut().ppu().dma_flag() {
        cpu.cycle_budget -= DMA_CPU_CYCLE_COUNT;
    }

    if let Some(pixels) = cpu.bus.borrow_mut().ppu().get_image() {
        // TODO: We should do this on a fixed interval runtime but threading in tokio is going
        // to be a hassle..
        // std::thread::sleep(sleep_duration);

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