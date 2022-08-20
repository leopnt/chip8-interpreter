#[forbid(unsafe_code)]
mod display;
mod interpreter;
mod keyconf;
mod memory;

use display::Display;
use interpreter::Interpreter;
use memory::Memory;

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

use std::time::Instant;

#[macro_use]
extern crate lazy_static;

fn main() {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let mut display = Display::new(&event_loop);

    let mut memory = Memory::new();
    let mut interpreter = Interpreter::new();

    let font = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];

    memory.load_font(&font);

    let program_path = std::env::args()
        .nth(1)
        .expect("Please give path to .ch8 file");
    let program = std::fs::read(program_path).unwrap();

    memory.load_prog(&program);

    let mut start = Instant::now();
    let mut delta: f32 = 0.0;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                display.pixels.resize_surface(size.width, size.height);
            }

            interpreter.apply_input(&input);
        }

        interpreter.decrement_timers();
        interpreter.step(&mut memory);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                display.draw(&memory);

                if display
                    .pixels
                    .render()
                    .map_err(|e| println!("pixels.render() failed: {}", e))
                    .is_err()
                {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                display.window().request_redraw();

                let elapsed = start.elapsed();
                delta = (elapsed.as_micros() as f32) / 1000_000.0;
                start = Instant::now();
            }
            _ => (),
        }
    })
}
