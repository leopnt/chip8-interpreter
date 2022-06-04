use crate::memory::Memory;

const STACK_SIZE: u8 = 0xff;
const NUM_REGISTERS: u8 = 16;

pub struct Interpreter {
    stack: [u8; STACK_SIZE as usize], // stack is here instead of in-memory
    vi: u16,                          // index register
    vx: [u8; NUM_REGISTERS as usize], // registers V0 to VF
    pc: u16,                          // program counter
    dt: u8,                           // delay timer
    st: u8,                           // sound timer
    interupt: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            vi: 0,
            vx: [0; NUM_REGISTERS as usize],
            pc: 0x0200,
            dt: 0,
            st: 0,
            stack: [0; STACK_SIZE as usize],
            interupt: false,
        }
    }

    pub fn set_vx(&mut self, x: u8, data: u8) {
        self.vx[x as usize] = data;
    }

    pub fn step(&mut self, memory: &mut Memory) {
        let opcode = self.next(memory);
        self.pc += 2;
        self.exec(opcode, memory);
    }

    pub fn next(&self, mem: &Memory) -> u16 {
        mem.read_u16(self.pc)
    }

    fn exec(&mut self, opcode: u16, memory: &mut Memory) {
        if opcode == 0x0000 {
            self.interupt = true;
            return;
        }

        match Interpreter::mode(opcode) {
            0x1 => {
                let nnn = Interpreter::nnn(opcode);
                self.pc = nnn;
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

            // set index register
            0xA => {
                let nnn = Interpreter::nnn(opcode);
                self.vi = nnn;
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

        while !interpreter.interupt {
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

        while !interpreter.interupt {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xC1, interpreter.vx[0]);
    }

    #[test]
    fn test_set_vi() {
        let mut mem = Memory::new();
        mem.load_prog(&[0xAC, 0xC0, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.interupt {
            interpreter.step(&mut mem);
        }

        assert_eq!(0xCC0, interpreter.vi);
    }

    #[test]
    fn test_jump() {
        let mut mem = Memory::new();
        mem.load_prog(&[0x12, 0x04, 0x55, 0x55, 0x00, 0x00]);
        let mut interpreter = Interpreter::new();

        while !interpreter.interupt {
            interpreter.step(&mut mem);
        }

        assert_eq!(0x0206, interpreter.pc);
    }
}
