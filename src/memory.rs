const MAX_SIZE: u16 = 0x1000;

const PROG_LOC: u16 = 0x0200;
pub const DISPLAY_LOC: u16 = 0x0F00;
pub const FONT_LOC: u16 = 0x0050;
pub const FONT_CHAR_SIZE: u16 = 5; // bytes

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

    pub fn load_font(&mut self, font: &[u8]) {
        for (i, byte) in font.iter().enumerate() {
            self.data[(FONT_LOC as usize + i)] = *byte;
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

    #[test]
    fn test_load_font() {
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

        for i in 0..font.len() {
            assert_eq!(font[i], mem.read(FONT_LOC + i as u16));
        }
    }
}
