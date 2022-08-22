use winit::event::VirtualKeyCode;

use std::collections::HashMap;

/* COSMAC VIP keys */
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
#[repr(usize)]
pub enum COSMACVIP {
    KEY0 = 0,
    KEY1 = 1,
    KEY2 = 2,
    KEY3 = 3,
    KEY4 = 4,
    KEY5 = 5,
    KEY6 = 6,
    KEY7 = 7,
    KEY8 = 8,
    KEY9 = 9,
    KEYA = 10,
    KEYB = 11,
    KEYC = 12,
    KEYD = 13,
    KEYE = 14,
    KEYF = 15,
}

lazy_static! {
    pub static ref KEYCONFIG: HashMap<COSMACVIP, VirtualKeyCode> = {
        let mut m = HashMap::new();
        m.insert(COSMACVIP::KEY0, VirtualKeyCode::X);
        m.insert(COSMACVIP::KEY1, VirtualKeyCode::Key1);
        m.insert(COSMACVIP::KEY2, VirtualKeyCode::Key2);
        m.insert(COSMACVIP::KEY3, VirtualKeyCode::Key3);
        m.insert(COSMACVIP::KEY4, VirtualKeyCode::Q);
        m.insert(COSMACVIP::KEY5, VirtualKeyCode::W);
        m.insert(COSMACVIP::KEY6, VirtualKeyCode::E);
        m.insert(COSMACVIP::KEY7, VirtualKeyCode::A);
        m.insert(COSMACVIP::KEY8, VirtualKeyCode::S);
        m.insert(COSMACVIP::KEY9, VirtualKeyCode::D);
        m.insert(COSMACVIP::KEYA, VirtualKeyCode::Z);
        m.insert(COSMACVIP::KEYB, VirtualKeyCode::C);
        m.insert(COSMACVIP::KEYC, VirtualKeyCode::Key4);
        m.insert(COSMACVIP::KEYD, VirtualKeyCode::R);
        m.insert(COSMACVIP::KEYE, VirtualKeyCode::F);
        m.insert(COSMACVIP::KEYF, VirtualKeyCode::V);
        m
    };
    pub static ref COUNT: usize = KEYCONFIG.len();
}
