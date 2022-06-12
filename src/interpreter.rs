use crate::display::Display;
use crate::keyconf::{COSMACVIP, KEYCONFIG};
use crate::memory;
use crate::memory::Memory;

use winit_input_helper::WinitInputHelper;

const STACK_SIZE: usize = 0xff;
const NUM_REGISTERS: usize = 16;
const NUM_KEYS: usize = 16;

pub struct Interpreter {
    stack: [u16; STACK_SIZE], // stack is here instead of in-memory
    sc: u8,                   // stack counter
    vi: u16,                  // index register
    vx: [u8; NUM_REGISTERS],  // registers V0 to VF
    pub pc: u16,              // program counter
    dt: u8,                   // delay timer
    st: u8,                   // sound timer
    key_held: [bool; NUM_KEYS],
    stop: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            vi: 0,
            vx: [0; NUM_REGISTERS],
            pc: 0x0200,
            dt: 0,
            st: 0,
            stack: [0; STACK_SIZE],
            sc: 0,
            key_held: [false; NUM_KEYS],
            stop: false,
        }
    }

    fn set_vx(&mut self, x: u8, data: u8) {
        self.vx[x as usize] = data;
    }

    fn set_vf(&mut self, data: u8) {
        self.vx[15] = data;
    }

    pub fn vf(&self) -> u8 {
        self.vx[15]
    }

    pub fn decrement_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            self.st -= 1;
        }
    }

    fn set_dt(&mut self, value: u8) {
        self.dt = value;
    }

    fn set_st(&mut self, value: u8) {
        self.st = value;
    }

    pub fn apply_input(&mut self, input: &WinitInputHelper) {
        self.key_held = [false; NUM_KEYS]; // reset keys

        for (key, virtualkeycode) in KEYCONFIG.iter() {
            if input.key_held(*virtualkeycode) {
                self.key_held[*key as usize] = true;
            }
        }
    }

    pub fn stop(&self) -> bool {
        self.stop
    }

    pub fn step(&mut self, memory: &mut Memory) {
        let opcode = self.next(memory);
        self.pc += 2;
        self.exec(opcode, memory);
    }

    pub fn next(&self, mem: &Memory) -> u16 {
        mem.read_u16(self.pc)
    }

    pub fn stack_push(&mut self, value: u16) {
        self.stack[self.sc as usize] = value;
        self.sc += 1;
    }

    pub fn stack_pop(&mut self) -> u16 {
        self.sc -= 1;
        self.stack[self.sc as usize]
    }

    fn exec(&mut self, opcode: u16, memory: &mut Memory) {
        if opcode == 0x0000 {
            self.stop = true;
            return;
        }

        match Interpreter::mode(opcode) {
            0x0 => {
                let nnn = Interpreter::nnn(opcode);
                match nnn {
                    // clear screen
                    0x0E0 => {
                        for pixel_addr in 0x00..0xFF {
                            memory.write(memory::DISPLAY_LOC + pixel_addr, 0);
                        }
                    }
                    0x0EE => {
                        self.pc = self.stack_pop();
                    }

                    _ => panic!("Unkown opcode"),
                }
            }

            // jump
            0x1 => {
                let nnn = Interpreter::nnn(opcode);
                self.pc = nnn;
            }

            // subroutines
            0x2 => {
                self.stack_push(self.pc);
                self.pc = Interpreter::nnn(opcode);
            }

            // skip if VX == nn
            0x3 => {
                let x = Interpreter::x(opcode);
                let nn = Interpreter::nn(opcode);
                if self.vx[x as usize] == nn {
                    self.pc += 2;
                }
            }

            // skip if VX != nn
            0x4 => {
                let x = Interpreter::x(opcode);
                let nn = Interpreter::nn(opcode);
                if self.vx[x as usize] != nn {
                    self.pc += 2;
                }
            }

            // skip if VX == VY
            0x5 => {
                let n = Interpreter::n(opcode);
                if n != 0 {
                    panic!("Unknown instruction");
                }

                let x = Interpreter::x(opcode);
                let y = Interpreter::y(opcode);
                if self.vx[x as usize] == self.vx[y as usize] {
                    self.pc += 2;
                }
            }

            // set register VX
            0x6 => {
                let x = Interpreter::x(opcode);
                let nn = Interpreter::nn(opcode);
                self.set_vx(x, nn)
            }

            // add value to vx
            0x7 => {
                let x = Interpreter::x(opcode);
                let nn = Interpreter::nn(opcode);
                let vx = self.vx[x as usize];
                self.set_vx(x, vx.wrapping_add(nn));
            }

            // skip if VX != VY
            0x9 => {
                let n = Interpreter::n(opcode);
                if n != 0 {
                    panic!("Unknown instruction");
                }

                let x = Interpreter::x(opcode);
                let y = Interpreter::y(opcode);
                if self.vx[x as usize] != self.vx[y as usize] {
                    self.pc += 2;
                }
            }

            // set index register
            0xA => {
                let nnn = Interpreter::nnn(opcode);
                self.vi = nnn;
            }

            // draw to screen
            0xD => {
                let x = Interpreter::x(opcode);
                let y = Interpreter::y(opcode);
                let n = Interpreter::n(opcode);

                let vx = self.vx[x as usize];
                let vy = self.vx[y as usize];

                let mut row = 0;
                for sprite_byte_addr in self.vi..(self.vi + n as u16) {
                    let mut col = 0;

                    let sprite_byte = memory.read(sprite_byte_addr);

                    for sprite_bit_idx in 0..8 {
                        let sprite_bit = (sprite_byte >> (7 - sprite_bit_idx)) & 0b0000_0001;

                        if sprite_bit == 1 {
                            let pos_x = vx + col;
                            let pos_y = vy + row;
                            // don't display if outside of the screen
                            if pos_x < 64 && pos_y < 32 {
                                let curr_pixel = Display::read_pixel(memory, pos_x, pos_y);

                                // pixel collision
                                if curr_pixel == 1 {
                                    self.set_vf(1);
                                }

                                Display::write_pixel(memory, pos_x, pos_y);
                            }
                        }

                        col += 1;
                    }

                    row += 1;
                }
            }

            _ => panic!("Unknown mode"),
        }
    }

    fn mode(opcode: u16) -> u8 {
        ((opcode & 0b1111_0000_0000_0000) >> 12) as u8
    }

    fn x(opcode: u16) -> u8 {
        ((opcode & 0b1111_0000_0000) >> 8) as u8
    }

    fn y(opcode: u16) -> u8 {
        ((opcode & 0b0000_1111_0000) >> 4) as u8
    }

    fn n(opcode: u16) -> u8 {
        (opcode & 0b0000_0000_1111) as u8
    }

    fn nn(opcode: u16) -> u8 {
        (opcode & 0b0000_1111_1111) as u8
    }

    fn nnn(opcode: u16) -> u16 {
        opcode & 0b1111_1111_1111
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_vx() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x60, 0xC0, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xC0, interpreter.vx[0]);
    }

    #[test]
    fn test_add_to_vx() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x70, 0x01, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();
        interpreter.vx[0] = 0xC0;

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xC1, interpreter.vx[0]);
    }

    #[test]
    fn test_set_vi() {
        let mut mem = Memory::new();
        mem.load_prog(&[0xAC, 0xC0, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }

    #[test]
    fn test_jump() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x12, 0x04, 0x55, 0x55, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x0206, interpreter.pc);
    }

    #[test]
    fn test_clear_screen() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x00, 0xE0, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        Display::write_pixel(&mut mem, 2, 3);

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        for pixel_addr in 0x00..0xFF {
            assert_eq!(0, mem.read(memory::DISPLAY_LOC + pixel_addr));
        }
    }

    #[test]
    fn test_display() {
        let mut mem = Memory::new();

        /*
        sprite is at addr 0x020A (I set to 0x020A)
        it looks like
        11111111
        11110000
        11010101
        11111111

        we draw it at x = 1 and y = 2 (V0 set to 1 and V1 set to 2)
        we draw only the first two bytes (DXY2) of the sprite
        */

        // manually write the pixel at 2, 3 (this is on the location that will
        // be written) to cause a collision
        // this is to check that VF is equal to 1 after the display instruction
        Display::write_pixel(&mut mem, 2, 3);

        mem.load_prog(&[
            0xA2, 0x0A, 0x60, 0x01, 0x61, 0x02, 0xD0, 0x12, 0x00, 0x00, 0b11111111, 0b11110000,
            0b11010101, 0b11111111,
        ]);

        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        // check collision has set VF
        assert_eq!(interpreter.vf(), 1);

        for row in 0..32 {
            for col in 0..64 {
                // this is where collision happens
                if col == 2 && row == 3 {
                    assert_eq!(Display::read_pixel(&mem, col, row), 0);
                    continue;
                }

                // first byte of sprite
                if col >= 1 && col <= 8 && row == 2 {
                    assert_eq!(Display::read_pixel(&mem, col, row), 1);
                // second byte of sprite
                } else if col >= 1 && col <= 4 && row == 3 {
                    assert_eq!(Display::read_pixel(&mem, col, row), 1);
                } else {
                    assert_eq!(Display::read_pixel(&mem, col, row), 0);
                }
            }
        }
    }

    #[test]
    fn test_subroutines() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x00, 0xE0, 0x22, 0x06, 0x00, 0x00, 0xA0, 0xC0, 0x00, 0xEE]); // clear screen, jump to 0x0206 subroutine, set VI and return
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x0206, interpreter.pc);
        assert_eq!(0x0C0, interpreter.vi);
    }

    #[test]
    fn test_skip_if_vx_equals_nn() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xAA, // set V0
            0x30, 0xAA, 0x00, 0x00, // skip next if V0 == 0xAA
            0x40, 0xBB, 0x00, 0x00, // skip next if V0 != 0xBB
            0xAC, 0xC0, 0x00, 0x00, // set VI to 0xCC0
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }

    #[test]
    fn test_skip_if_vx_equals_vy() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xAA, 0x61, 0xAA, // set V0, V1
            0x50, 0x10, 0x00, 0x00, // skip next if V0 == V1
            0x60, 0xAA, 0x61, 0xBB, // set V0, V1
            0x90, 0x10, 0x00, 0x00, // skip next if V0 != V1
            0xAC, 0xC0, 0x00, 0x00, // set VI to 0xCC0
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }
}
