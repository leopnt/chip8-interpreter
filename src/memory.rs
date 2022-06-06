const MAX_SIZE: u16 = 0x1000;

const PROG_LOC: u16 = 0x0200;
pub const DISPLAY_LOC: u16 = 0x0F00;

pub struct Memory {
    data: [u8; MAX_SIZE as usize],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            data: [0; MAX_SIZE as usize],
        }
    }

    pub fn hexdump(&self, from: u16, len: u16) {
        print!("hexdump from 0x{:04x}: ", from - from % 2);
        for addr in (from - from % 2)..(from + len) {
            if (addr) % 0x10 == 0 {
                print!("\n{:04x}: ", addr);
            }
            if addr % 2 == 0 {
                print!("{:04x} ", self.read_u16(addr));
            }
        }
    }

    pub fn load_prog(&mut self, prgm: &[u8]) {
        for (i, byte) in prgm.iter().enumerate() {
            self.data[(PROG_LOC as usize + i)] = *byte;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;

        lo << 8 | hi
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    pub fn write_u16(&mut self, addr: u16, data: u16) {
        let lo = (data >> 8) as u8;
        let hi = data as u8;

        self.write(addr, lo);
        self.write(addr + 1, hi);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write() {
        let mut mem = Memory::new();
        mem.write(0x0004, 0xC0);
        assert_eq!(0xC0, mem.read(0x0004));
    }

    #[test]
    fn test_read_write_u16() {
        let mut mem = Memory::new();
        mem.write_u16(0x0004, 0xC042);
        assert_eq!(0xC042, mem.read_u16(0x0004));
    }

    #[test]
    fn test_load_prgm() {
        let data = [0x01, 0x02, 0x42, 0x04];
        let mut mem = Memory::new();
        mem.load_prog(&data);

        for i in 0..data.len() {
            assert_eq!(data[i], mem.read(PROG_LOC + i as u16));
        }
    }
}
