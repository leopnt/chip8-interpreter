use crate::display::Display;
use crate::keyconf::{COSMACVIP, KEYCONFIG};
use crate::memory;
use crate::memory::Memory;

use winit_input_helper::WinitInputHelper;

use rand::Rng;

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

    /// Returns the index of the pressed key if there is one (the first in the array)
    pub fn get_first_key_pressed(&self) -> Option<usize> {
        for (key_idx, &pressed) in self.key_held.iter().enumerate() {
            if pressed {
                return Some(key_idx);
            }
        }

        None
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

            // logical and arithmetic instructions
            0x8 => {
                let n = Interpreter::n(opcode);
                let x = Interpreter::x(opcode);
                let y = Interpreter::y(opcode);

                let vx = self.vx[x as usize];
                let vy = self.vx[y as usize];

                match n {
                    // set VX to the value of VY
                    0x0 => {
                        self.set_vx(x, vy);
                    }

                    // binary OR
                    0x1 => {
                        self.set_vx(x, vx | vy);
                    }

                    // binary AND
                    0x2 => {
                        self.set_vx(x, vx & vy);
                    }

                    // logical XOR
                    0x3 => {
                        self.set_vx(x, vx ^ vy);
                    }

                    // add
                    0x4 => {
                        let overflows = vx.checked_add(vy).is_none() as u8;

                        self.set_vx(x, vx.wrapping_add(vy));
                        self.set_vf(overflows);
                    }

                    // substract VX - VY
                    0x5 => {
                        let underflows = vx.checked_sub(vy).is_none() as u8;

                        self.set_vx(x, vx.wrapping_sub(vy));
                        self.set_vf(1 - underflows); // 0 if underflows else 1
                    }

                    // substract VY - VX
                    0x7 => {
                        let underflows = vy.checked_sub(vx).is_none() as u8;

                        self.set_vx(x, vy.wrapping_sub(vx));
                        self.set_vf(1 - underflows); // 0 if underflows else 1
                    }

                    // shift 1 bit to the right
                    0x6 => {
                        // TODO: optional of configurable: set vx to vy
                        let shifted_bit = vx & 0b0000_0001;
                        self.set_vx(x, vx >> 1);
                        self.set_vf(shifted_bit);
                    }

                    // shift 1 bit to the left
                    0xE => {
                        // TODO: optional of configurable: set vx to vy
                        let shifted_bit = (vx & 0b1000_0000) >> 7;
                        self.set_vx(x, vx << 1);
                        self.set_vf(shifted_bit);
                    }

                    _ => panic!("Unknown N for instruction: 0x8XYN"),
                }
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

            // jump with offset
            0xB => {
                // TODO: configurable instruction with BXNN (see SUPER-CHIP)
                let nnn = Interpreter::nnn(opcode);
                self.pc = nnn + self.vx[0] as u16;
            }

            // random
            0xC => {
                let nn = Interpreter::nn(opcode);
                let x = Interpreter::x(opcode);

                let r: u8 = rand::thread_rng().gen();

                self.set_vx(x, r & nn);
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

            // skip if key
            0xE => {
                let x = Interpreter::x(opcode);
                let vx = self.vx[x as usize];

                let is_key_pressed_at_vx = self.key_held[vx as usize];

                let nn = Interpreter::nn(opcode);

                match nn {
                    0x9E => {
                        if is_key_pressed_at_vx {
                            self.pc += 2;
                        }
                    }

                    0xA1 => {
                        if !is_key_pressed_at_vx {
                            self.pc += 2;
                        }
                    }

                    _ => panic!("Unknown NN for instruction: 0xEXNN"),
                }
            }

            // miscellaneous
            0xF => {
                let x = Interpreter::x(opcode);
                let vx = self.vx[x as usize];

                let nn = Interpreter::nn(opcode);

                match nn {
                    // read delay timer to vx
                    0x07 => self.set_vx(x, self.dt),

                    // set delay timer to vx
                    0x15 => self.set_dt(vx),

                    // set sound timer to vx
                    0x18 => self.set_st(vx),

                    // add to index
                    0x1E => self.vi = self.vi.wrapping_add(vx as u16),

                    // get key
                    0x0A => {
                        let first_key_pressed = self.get_first_key_pressed();
                        if first_key_pressed.is_some() {
                            self.set_vx(x, first_key_pressed.unwrap() as u8);
                        }
                        // go back (e.g. loop) until key press
                        else {
                            self.pc -= 2;
                        }
                    }

                    // font character
                    0x29 => {
                        let x = Interpreter::x(opcode);
                        let vx = self.vx[x as usize];

                        let offset = (vx as u16) * memory::FONT_CHAR_SIZE;
                        self.vi = memory::FONT_LOC + offset;
                    }

                    // binary-coded decimal conversion
                    0x33 => {
                        let x = Interpreter::x(opcode);
                        let vx = self.vx[x as usize];

                        let right_digit = (vx / 1) % 10;
                        let mid_digit = (vx / 10) % 10;
                        let left_digit = (vx / 100) % 10;

                        memory.write(self.vi, left_digit);
                        memory.write(self.vi + 1, mid_digit);
                        memory.write(self.vi + 2, right_digit);
                    }

                    // write register to mem
                    0x55 => {
                        // TODO: configurable instruction
                        let x_max = Interpreter::x(opcode);
                        for x in 0..(x_max + 1) {
                            let addr = self.vi + x as u16;
                            let value = self.vx[x as usize];
                            memory.write(addr, value);
                        }
                    }

                    // read mem to registers
                    0x65 => {
                        // TODO: configurable instruction
                        let x_max = Interpreter::x(opcode);
                        for x in 0..(x_max + 1) {
                            let addr = self.vi + x as u16;
                            self.vx[x as usize] = memory.read(addr);
                        }
                    }

                    _ => panic!("Unknown NN for instruction: 0xFXNN"),
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
        mem.load_prog(&[0x12, 0x04, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x0206, interpreter.pc);
    }

    #[test]
    fn test_jump_with_offset() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x01, // set V0 to 0x01
            0xB2, 0x04, // jump to 0x204 + V0
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x0207, interpreter.pc);
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

    #[test]
    fn test_logical_arithmetic_set() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xAA, 0x61, 0xBB, // set V0, V1
            0x80, 0x10, 0x00, 0x00, // set V0 to V1
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xBB, interpreter.vx[0]);
    }

    #[test]
    fn test_logical_arithmetic_binary_or() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x00, 0x61, 0x00, // set V0, V1
            0x62, 0x00, 0x63, 0x01, // set V2, V3
            0x64, 0x01, 0x65, 0x00, // set V4, V5
            0x66, 0x01, 0x67, 0x01, // set V6, V7
            0x80, 0x11, 0x82, 0x31, // V0 = V0 | V1, V2 = V2 | V3
            0x84, 0x51, 0x86, 0x71, // V4 = V4 | V5, V6 = V6 | V7
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x00, interpreter.vx[0]);
        assert_eq!(0x01, interpreter.vx[2]);
        assert_eq!(0x01, interpreter.vx[4]);
        assert_eq!(0x01, interpreter.vx[6]);
    }

    #[test]
    fn test_logical_arithmetic_binary_and() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x00, 0x61, 0x00, // set V0, V1
            0x62, 0x00, 0x63, 0x01, // set V2, V3
            0x64, 0x01, 0x65, 0x00, // set V4, V5
            0x66, 0x01, 0x67, 0x01, // set V6, V7
            0x80, 0x12, 0x82, 0x32, // V0 = V0 & V1, V2 = V2 & V3
            0x84, 0x52, 0x86, 0x72, // V4 = V4 & V5, V6 = V6 & V7
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x00, interpreter.vx[0]);
        assert_eq!(0x00, interpreter.vx[2]);
        assert_eq!(0x00, interpreter.vx[4]);
        assert_eq!(0x01, interpreter.vx[6]);
    }

    #[test]
    fn test_logical_arithmetic_binary_xor() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x00, 0x61, 0x00, // set V0, V1
            0x62, 0x00, 0x63, 0x01, // set V2, V3
            0x64, 0x01, 0x65, 0x00, // set V4, V5
            0x66, 0x01, 0x67, 0x01, // set V6, V7
            0x80, 0x13, 0x82, 0x33, // V0 = V0 ^ V1, V2 = V2 ^ V3
            0x84, 0x53, 0x86, 0x73, // V4 = V4 ^ V5, V6 = V6 ^ V7
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x00, interpreter.vx[0]);
        assert_eq!(0x01, interpreter.vx[2]);
        assert_eq!(0x01, interpreter.vx[4]);
        assert_eq!(0x00, interpreter.vx[6]);
    }

    #[test]
    fn test_logical_arithmetic_and() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xFC, 0x61, 0x03, // set V0, V1
            0x80, 0x14, 0x80, 0x14, // V0 = V0 + V1, V0 = V0 + V1 -> overflow
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x02, interpreter.vx[0]);
        assert_eq!(0x01, interpreter.vf());
    }

    #[test]
    fn test_logical_arithmetic_sub() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xFF, 0x61, 0x01, // set V0, V1
            0x62, 0x02, 0x63, 0x00, // set V2, V3
            0x80, 0x15, 0x82, 0x37, // V0 = V0 - V1, V2 = V3 - V2 -> underflow
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xFE, interpreter.vx[0]);
        assert_eq!(0xFE, interpreter.vx[2]);
        assert_eq!(0x00, interpreter.vf());
    }

    #[test]
    fn test_logical_arithmetic_bit_shift() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x02, 0x61, 0xFF, // set V0, V1
            0x80, 0x06, 0x81, 0x1E, // V0 = V0 >> 1, V1 = V1 << 1
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0b0000_0001, interpreter.vx[0]);
        assert_eq!(0b1111_1110, interpreter.vx[1]);
        assert_eq!(0x01, interpreter.vf());
    }

    #[test]
    fn test_skip_if_key_pressed() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x0A, // set V0
            0xE0, 0x9E, 0x00, 0x00, // skip next if V0 == 0xAA
            0xAC, 0xC0, 0x00, 0x00, // set VI to 0xCC0
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            // emulate key 0x0A pressed
            interpreter.key_held[0x0A] = true;
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }

    #[test]
    fn test_skip_if_key_not_pressed() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x0A, // set V0
            0xE0, 0xA1, 0x00, 0x00, // skip next if V0 == 0xAA
            0xAC, 0xC0, 0x00, 0x00, // set VI to 0xCC0
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            // emulate key 0x0A not pressed
            interpreter.key_held[0x0A] = false;
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }

    #[test]
    fn test_set_timers() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x0A, // set V0
            0xF0, 0x15, 0xF0, 0x18, // delay timer = V0, sound timer = V0
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        interpreter.decrement_timers();

        assert_eq!(0x0A - 1, interpreter.dt);
        assert_eq!(0x0A - 1, interpreter.st);
    }

    #[test]
    fn test_read_delay_timer() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0xAA, // set V0 to 0xAA
            0xF0, 0x07, // set V0 to delay timer
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x00, interpreter.vx[0]);
    }

    #[test]
    fn test_add_to_index() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0xAC, 0xC0, // set VI to 0xCC0
            0x60, 0x02, // set V0 to 0x02
            0xF0, 0x1E, // VI = VI + V0
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC2, interpreter.vi);
    }

    #[test]
    fn test_font_character() {
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

        let mut mem = Memory::new();
        mem.load_font(&font);

        mem.load_prog(&[
            0x60, 0x0A, // set V0 to 0x0A
            0xF0, 0x29, // VI = mem addr of character of hex at V0
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        // hex(0x050 + 5 * 0x0A) = 0x82
        assert_eq!(0x82, interpreter.vi);
        assert_eq!(0xF0, mem.read(interpreter.vi));
    }

    #[test]
    fn test_binary_coded_decimal_conversion() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x9C, // set V0 to 0x9C (= 156)
            0xA5, 0x00, // VI = 0x500
            0xF0, 0x33, // mem write V0 at addr VI
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(1, mem.read(0x500));
        assert_eq!(5, mem.read(0x501));
        assert_eq!(6, mem.read(0x502));
    }

    #[test]
    fn test_mem_write_registers() {
        let mut mem = Memory::new();
        mem.load_prog(&[
            0x60, 0x9C, // set V0 to 0x9C
            0x61, 0x9D, // set V0 to 0x9D
            0x62, 0x9E, // set V0 to 0x9E
            0xA5, 0x00, // VI = 0x500
            0xF2, 0x55, // mem write V0..(V2 + 1) at addr VI
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x9C, mem.read(0x500));
        assert_eq!(0x9D, mem.read(0x501));
        assert_eq!(0x9E, mem.read(0x502));
        assert_eq!(0x00, mem.read(0x503));
    }

    #[test]
    fn test_registers_read_mem() {
        let mut mem = Memory::new();
        mem.write(0x500, 0x9C);
        mem.write(0x501, 0x9D);
        mem.write(0x502, 0x9E);

        mem.load_prog(&[
            0xA5, 0x00, // VI = 0x500
            0xF2, 0x65, // mem read V0..(V2 + 1) at addr VI
            0x00, 0x00,
        ]);
        let mut interpreter = Interpreter::new();

        while !interpreter.stop() {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x9C, interpreter.vx[0]);
        assert_eq!(0x9D, interpreter.vx[1]);
        assert_eq!(0x9E, interpreter.vx[2]);
        assert_eq!(0x00, interpreter.vx[3]);
    }
}
