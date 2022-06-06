use crate::memory;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct Display {
    pub pixels: Pixels,
    window: Window,
}

impl Display {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = {
            let size = LogicalSize::new(512, 256);
            WindowBuilder::new()
                .with_title("CHIP-8")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)
                .unwrap()
        };

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(64, 32, surface_texture).unwrap()
        };

        Display { window, pixels }
    }

    pub fn read_pixel(memory: &memory::Memory, x: u8, y: u8) -> u8 {
        let byte = memory.read(Display::pos_to_byte_addr(x, y));
        let bit = byte >> (7 - Display::pos_to_bit_offset(x, y));

        return bit & 0b0000_0001;
    }

    pub fn write_pixel(memory: &mut memory::Memory, x: u8, y: u8) {
        let byte_addr = Display::pos_to_byte_addr(x, y);
        let bit_offset = Display::pos_to_bit_offset(x, y);

        let byte_to_write = 0b1000_0000 >> bit_offset;
        let current_byte = memory.read(byte_addr);

        memory.write(byte_addr, current_byte ^ byte_to_write);
    }

    pub fn pos_to_byte_addr(x: u8, y: u8) -> u16 {
        let bit_idx = Display::pos_to_bit_index(x, y);
        let byte_addr = bit_idx / 8;
        return memory::DISPLAY_LOC + byte_addr;
    }

    pub fn pos_to_bit_offset(x: u8, y: u8) -> u8 {
        Display::pos_to_bit_index(x, y) as u8 % 8
    }

    pub fn pos_to_bit_index(x: u8, y: u8) -> u16 {
        (x as u16) + (64 * (y as u16)) // x + DISPLAY_WIDTH * y
    }

    /// Modify texture pixels according to memory bits.
    /// Data is translated from binary values to array of RGBA values.
    /// Since the display is monochrome (0 or 1 in memory), we set the pixel to green (0x00FF00FF in the texture)
    pub fn draw(&mut self, memory: &memory::Memory) {
        let frame = self.pixels.get_frame();

        let mut byte_idx = 0;
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let bit_idx = i as u8 % 8;
            if bit_idx == 0 {
                byte_idx += 1
            }

            let byte = memory.read(memory::DISPLAY_LOC + byte_idx - 1);

            let bit = ((byte << bit_idx) & 0b1000_0000) >> 7;

            pixel.copy_from_slice(&[0x00, bit * 0xFF, 0x00, 0xFF]);
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
