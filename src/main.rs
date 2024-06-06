#![allow(unused)]
use anyhow::Result;

fn main() -> Result<()> {
    println!("8080 disassembler");

    let rom = std::fs::read("./rom/space-invaders/invaders").expect("Unable to read file");

    // let instructions = dissemble8080(&rom[0..32]);
    // dbg!(instructions);

    let mut cpu = Cpu8080::new();
    cpu.load(&rom);
    cpu.mirror = 0x400;

    dbg!(&cpu.memory[0..8]);

    cpu.step();
    cpu.step();
    cpu.step();
    cpu.step();

    dbg!(&cpu.history);

    // let rom = b"\x01\x02\x03";
    // cpu.load(rom);
    // cpu.step();
    // dbg!(&cpu.history);
    // dbg!(cpu.b);
    // dbg!(cpu.c);

    Ok(())
}

#[derive(Debug)]
struct Cpu8080 {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    /// program counter
    pub pc: u16,
    /// stack pointer
    pub sp: u16,

    /// zero
    pub z: bool,
    /// sign
    pub s: bool,
    /// parity
    pub p: bool,
    /// carry
    pub cy: bool,
    /// auxiliary carry
    pub ac: bool,

    pub int_enable: bool,

    pub halt: bool,

    pub memory: [u8; 0x10000],
    pub mirror: u16,

    pub history: Vec<String>,
}

macro_rules! single_reg_flag {
    ($reg:expr) => {{
        let z = $reg == 0;
        let s = $reg & (1 << 7) != 0;
        let p = $reg.count_ones() % 2 == 0;
        let ac = $reg & 0x0f > 9;
    }};
}

impl Cpu8080 {
    fn new() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            pc: 0,
            sp: 0,
            z: false,
            s: false,
            p: false,
            cy: false,
            ac: false,
            int_enable: false,
            halt: false,
            memory: [0; 0x10000],
            mirror: 0,
            history: Vec::new(),
        }
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    fn load(&mut self, rom: &[u8]) {
        self.memory[0..rom.len()].copy_from_slice(rom);
    }

    fn read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn next_memory(&self) -> u16 {
        self.read(self.pc + 1) as u16 | (self.read(self.pc + 2) as u16) << 8
    }

    fn step(&mut self) {
        match self.read(self.pc) {
            0x00 => self.history.push("NOP".to_string()),
            0x01 => {
                self.set_bc(self.next_memory());
                self.history
                    .push(format!("LXI B, {:#04x}{:#04x}", self.b, self.c));
            }
            0x02 => {
                self.memory[self.bc() as usize] = self.a;
                self.history.push("STAX B".to_string());
            }
            0x03 => {
                self.set_hl(self.hl().wrapping_add(1));
                self.history.push("INX B".to_string());
            }
            0x04 => {
                self.b = self.b.wrapping_add(1);
                self.z = self.b == 0;
                self.s = self.b & (1 << 7) != 0;
                self.p = self.b.count_ones() % 2 == 0;
                self.ac = self.b & 0x0f > 9;
                self.history.push("INR B".to_string());
            }
            0x05 => {
                self.b = self.b.wrapping_sub(1);
                self.z = self.b == 0;
                self.s = self.b & (1 << 7) != 0;
                self.p = self.b.count_ones() % 2 == 0;
                self.ac = self.b & 0x0f > 9;
                self.history.push("DCR B".to_string());
            }
            0x06 => {
                self.b = self.read(self.pc + 1);
                self.history.push(format!("MVI B, {:#04x}", self.b));
            }
            0x07 => {
                self.cy = self.a & (1 << 7) != 0;
                self.a = self.a.rotate_left(1);
                self.history.push("RLC".to_string());
            }
            0x08 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x09 => {
                let (hl, overflow) = self.hl().overflowing_add(self.bc());
                self.set_hl(hl);
                self.cy = overflow;
                self.history.push("DAD B".to_string());
            }
            0x0a => {
                self.a = self.memory[self.bc() as usize];
                self.history.push("LDAX B".to_string());
            }
            0x0b => {
                self.set_bc(self.bc().wrapping_sub(1));
                self.history.push("DCX B".to_string());
            }
            0x0c => {
                self.c = self.c.wrapping_add(1);
                self.z = self.c == 0;
                self.s = self.c & (1 << 7) != 0;
                self.p = self.c.count_ones() % 2 == 0;
                self.ac = self.c & 0x0f > 9;
                self.history.push("INR C".to_string());
            }
            0x0d => {
                self.c = self.c.wrapping_sub(1);
                self.z = self.c == 0;
                self.s = self.c & (1 << 7) != 0;
                self.p = self.c.count_ones() % 2 == 0;
                self.ac = self.c & 0x0f > 9;
                self.history.push("DCR C".to_string());
            }
            0x0e => {
                self.c = self.read(self.pc + 1);
                self.history.push(format!("MVI C, {:#04x}", self.c));
            }
            0x0f => {
                self.cy = self.a & (1 << 7) != 0;
                self.a = self.a.rotate_right(1);
                self.history.push("RRC".to_string());
            }
            0x10 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x11 => {
                self.set_de(self.next_memory());
                self.history
                    .push(format!("LXI D, {:#04x}{:#04x}", self.d, self.e));
            }
            0x12 => {
                self.memory[self.de() as usize] = self.a;
                self.history.push("STAX D".to_string());
            }
            0x13 => {
                self.set_de(self.de().wrapping_add(1));
                self.history.push("INX D".to_string());
            }
            0x14 => {
                self.d = self.d.wrapping_add(1);
                single_reg_flag!(self.d);

                // self.z = self.d == 0;
                // self.s = self.d & (1 << 7) != 0;
                // self.p = self.d.count_ones() % 2 == 0;
                // self.ac = self.d & 0x0f > 9;
                self.history.push("INR D".to_string());
            }
            0x15 => {
                self.d = self.d.wrapping_sub(1);
                self.z = self.d == 0;
                self.s = self.d & (1 << 7) != 0;
                self.p = self.d.count_ones() % 2 == 0;
                self.ac = self.d & 0x0f > 9;
                self.history.push("DCR D".to_string());
            }
            0x16 => {
                self.d = self.read(self.pc + 1);
                self.history.push(format!("MVI D, {:#04x}", self.d));
            }
            0x17 => {
                let cy = self.a & (1 << 7) != 0;
                self.a = self.a.rotate_left(1);
                self.a |= cy as u8;
                self.cy = cy;
                self.history.push("RAL".to_string());
            }
            0x18 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x19 => {
                let (hl, overflow) = self.hl().overflowing_add(self.de());
                self.set_hl(hl);
                self.cy = overflow;
                self.history.push("DAD D".to_string());
            }
            0x1a => {
                self.a = self.memory[self.de() as usize];
                self.history.push("LDAX D".to_string());
            }
            0x1b => {
                self.set_de(self.de().wrapping_sub(1));
                self.history.push("DCX D".to_string());
            }
            0x1c => {
                self.e = self.e.wrapping_add(1);
                self.z = self.e == 0;
                self.s = self.e & (1 << 7) != 0;
                self.p = self.e.count_ones() % 2 == 0;
                self.ac = self.e & 0x0f > 9;
                self.history.push("INR E".to_string());
            }
            0x1d => {
                self.e = self.e.wrapping_sub(1);
                self.z = self.e == 0;
                self.s = self.e & (1 << 7) != 0;
                self.p = self.e.count_ones() % 2 == 0;
                self.ac = self.e & 0x0f > 9;
                self.history.push("DCR E".to_string());
            }
            0x1e => {
                self.e = self.read(self.pc + 1);
                self.history.push(format!("MVI E, {:#04x}", self.e));
            }
            0x1f => {
                let cy = self.a & (1 << 7) != 0;
                self.a = self.a.rotate_right(1);
                self.a |= cy as u8;
                self.cy = cy;
                self.history.push("RAR".to_string());
            }
            0x20 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x21 => {
                self.set_hl(self.next_memory());
                self.history
                    .push(format!("LXI H, {:#04x}{:#04x}", self.h, self.l));
            }
            0x22 => {
                let addr = self.next_memory();
                self.memory[addr as usize] = self.l;
                self.memory[(addr + 1) as usize] = self.h;
                self.history.push(format!("SHLD {:#06x}", addr));
            }
            0x23 => {
                self.set_hl(self.hl().wrapping_add(1));
                self.history.push("INX H".to_string());
            }
            0x24 => {
                self.h = self.h.wrapping_add(1);
                self.z = self.h == 0;
                self.s = self.h & (1 << 7) != 0;
                self.p = self.h.count_ones() % 2 == 0;
                self.ac = self.h & 0x0f > 9;
                self.history.push("INR H".to_string());
            }
            0x25 => {
                self.h = self.h.wrapping_sub(1);
                self.z = self.h == 0;
                self.s = self.h & (1 << 7) != 0;
                self.p = self.h.count_ones() % 2 == 0;
                self.ac = self.h & 0x0f > 9;
                self.history.push("DCR H".to_string());
            }
            0x26 => {
                self.h = self.read(self.pc + 1);
                self.history.push(format!("MVI H, {:#04x}", self.h));
            }
            0x27 => {
                let cy = self.a & (1 << 7) != 0;
                let ac = self.a & 0x0f > 9;
                let a = self.a;
                self.a = self.a.rotate_left(1);
                self.a |= cy as u8;
                self.cy = cy;
                self.ac = ac;
                self.history.push("DAA".to_string());
            }
            0x28 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x29 => {
                let (hl, overflow) = self.hl().overflowing_add(self.hl());
                self.set_hl(hl);
                self.cy = overflow;
                self.history.push("DAD H".to_string());
            }
            0x2a => {
                let addr = self.next_memory();
                self.l = self.memory[addr as usize];
                self.h = self.memory[(addr + 1) as usize];
                self.history.push(format!("LHLD {:#06x}", addr));
            }
            0x2b => {
                self.set_hl(self.hl().wrapping_sub(1));
                self.history.push("DCX H".to_string());
            }
            0x2c => {
                self.l = self.l.wrapping_add(1);
                self.z = self.l == 0;
                self.s = self.l & (1 << 7) != 0;
                self.p = self.l.count_ones() % 2 == 0;
                self.ac = self.l & 0x0f > 9;
                self.history.push("INR L".to_string());
            }
            0x2d => {
                self.l = self.l.wrapping_sub(1);
                self.z = self.l == 0;
                self.s = self.l & (1 << 7) != 0;
                self.p = self.l.count_ones() % 2 == 0;
                self.ac = self.l & 0x0f > 9;
                self.history.push("DCR L".to_string());
            }
            0x2e => {
                self.l = self.read(self.pc + 1);
                self.history.push(format!("MVI L, {:#04x}", self.l));
            }
            0x2f => {
                self.a = !self.a;
                self.history.push("CMA".to_string());
            }
            0x30 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x31 => {
                self.sp = self.next_memory();
                self.history.push(format!("LXI SP, {:#06x}", self.sp));
            }
            0x32 => {
                let addr = self.next_memory();
                self.memory[addr as usize] = self.a;
                self.history.push(format!("STA {:#06x}", addr));
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                self.history.push("INX SP".to_string());
            }
            0x34 => {
                let addr = self.hl();
                self.memory[addr as usize] = self.memory[addr as usize].wrapping_add(1);
                self.z = self.memory[addr as usize] == 0;
                self.s = self.memory[addr as usize] & (1 << 7) != 0;
                self.p = self.memory[addr as usize].count_ones() % 2 == 0;
                self.ac = self.memory[addr as usize] & 0x0f > 9;
                self.history.push("INR M".to_string());
            }
            0x35 => {
                let addr = self.hl();
                self.memory[addr as usize] = self.memory[addr as usize].wrapping_sub(1);
                self.z = self.memory[addr as usize] == 0;
                self.s = self.memory[addr as usize] & (1 << 7) != 0;
                self.p = self.memory[addr as usize].count_ones() % 2 == 0;
                self.ac = self.memory[addr as usize] & 0x0f > 9;
                self.history.push("DCR M".to_string());
            }
            0x36 => {
                let addr = self.hl();
                self.memory[addr as usize] = self.read(self.pc + 1);
                self.history
                    .push(format!("MVI M, {:#04x}", self.memory[addr as usize]));
            }
            0x37 => {
                self.cy = true;
                self.history.push("STC".to_string());
            }
            0x38 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x39 => {
                let (hl, overflow) = self.hl().overflowing_add(self.sp);
                self.set_hl(hl);
                self.cy = overflow;
                self.history.push("DAD SP".to_string());
            }
            0x3a => {
                let addr = self.next_memory();
                self.a = self.memory[addr as usize];
                self.history.push(format!("LDA {:#06x}", addr));
            }
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                self.history.push("DCX SP".to_string());
            }
            0x3c => {
                self.a = self.a.wrapping_add(1);
                self.z = self.a == 0;
                self.s = self.a & (1 << 7) != 0;
                self.p = self.a.count_ones() % 2 == 0;
                self.ac = self.a & 0x0f > 9;
                self.history.push("INR A".to_string());
            }
            0x3d => {
                self.a = self.a.wrapping_sub(1);
                self.z = self.a == 0;
                self.s = self.a & (1 << 7) != 0;
                self.p = self.a.count_ones() % 2 == 0;
                self.ac = self.a & 0x0f > 9;
                self.history.push("DCR A".to_string());
            }
            0x3e => {
                self.a = self.read(self.pc + 1);
                self.history.push(format!("MVI A, {:#04x}", self.a));
            }
            0x3f => {
                self.a = !self.a;
                self.history.push("CMC".to_string());
            }
            0x40 => {
                self.b = self.b;
                self.history.push("MOV B, B".to_string());
            }
            0x41 => {
                self.b = self.c;
                self.history.push("MOV B, C".to_string());
            }
            0x42 => {
                self.b = self.d;
                self.history.push("MOV B, D".to_string());
            }
            0x43 => {
                self.b = self.e;
                self.history.push("MOV B, E".to_string());
            }
            0x44 => {
                self.b = self.h;
                self.history.push("MOV B, H".to_string());
            }
            0x45 => {
                self.b = self.l;
                self.history.push("MOV B, L".to_string());
            }
            0x46 => {
                self.b = self.memory[self.hl() as usize];
                self.history.push("MOV B, M".to_string());
            }
            0x47 => {
                self.b = self.a;
                self.history.push("MOV B, A".to_string());
            }
            0x48 => {
                self.c = self.b;
                self.history.push("MOV C, B".to_string());
            }
            0x49 => {
                self.c = self.c;
                self.history.push("MOV C, C".to_string());
            }
            0x4a => {
                self.c = self.d;
                self.history.push("MOV C, D".to_string());
            }
            0x4b => {
                self.c = self.e;
                self.history.push("MOV C, E".to_string());
            }
            0x4c => {
                self.c = self.h;
                self.history.push("MOV C, H".to_string());
            }
            0x4d => {
                self.c = self.l;
                self.history.push("MOV C, L".to_string());
            }
            0x4e => {
                self.c = self.memory[self.hl() as usize];
                self.history.push("MOV C, M".to_string());
            }
            0x4f => {
                self.c = self.a;
                self.history.push("MOV C, A".to_string());
            }
            0x50 => {
                self.d = self.b;
                self.history.push("MOV D, B".to_string());
            }
            0x51 => {
                self.d = self.c;
                self.history.push("MOV D, C".to_string());
            }
            0x52 => {
                self.d = self.d;
                self.history.push("MOV D, D".to_string());
            }
            0x53 => {
                self.d = self.e;
                self.history.push("MOV D, E".to_string());
            }
            0x54 => {
                self.d = self.h;
                self.history.push("MOV D, H".to_string());
            }
            0x55 => {
                self.d = self.l;
                self.history.push("MOV D, L".to_string());
            }
            0x56 => {
                self.d = self.memory[self.hl() as usize];
                self.history.push("MOV D, M".to_string());
            }
            0x57 => {
                self.d = self.a;
                self.history.push("MOV D, A".to_string());
            }
            0x58 => {
                self.e = self.b;
                self.history.push("MOV E, B".to_string());
            }
            0x59 => {
                self.e = self.c;
                self.history.push("MOV E, C".to_string());
            }
            0x5a => {
                self.e = self.d;
                self.history.push("MOV E, D".to_string());
            }
            0x5b => {
                self.e = self.e;
                self.history.push("MOV E, E".to_string());
            }
            0x5c => {
                self.e = self.h;
                self.history.push("MOV E, H".to_string());
            }
            0x5d => {
                self.e = self.l;
                self.history.push("MOV E, L".to_string());
            }
            0x5e => {
                self.e = self.memory[self.hl() as usize];
                self.history.push("MOV E, M".to_string());
            }
            0x5f => {
                self.e = self.a;
                self.history.push("MOV E, A".to_string());
            }
            0x60 => {
                self.h = self.b;
                self.history.push("MOV H, B".to_string());
            }
            0x61 => {
                self.h = self.c;
                self.history.push("MOV H, C".to_string());
            }
            0x62 => {
                self.h = self.d;
                self.history.push("MOV H, D".to_string());
            }
            0x63 => {
                self.h = self.e;
                self.history.push("MOV H, E".to_string());
            }
            0x64 => {
                self.h = self.h;
                self.history.push("MOV H, H".to_string());
            }
            0x65 => {
                self.h = self.l;
                self.history.push("MOV H, L".to_string());
            }
            0x66 => {
                self.h = self.memory[self.hl() as usize];
                self.history.push("MOV H, M".to_string());
            }
            0x67 => {
                self.h = self.a;
                self.history.push("MOV H, A".to_string());
            }
            0x68 => {
                self.l = self.b;
                self.history.push("MOV L, B".to_string());
            }
            0x69 => {
                self.l = self.c;
                self.history.push("MOV L, C".to_string());
            }
            0x6a => {
                self.l = self.d;
                self.history.push("MOV L, D".to_string());
            }
            0x6b => {
                self.l = self.e;
                self.history.push("MOV L, E".to_string());
            }
            0x6c => {
                self.l = self.h;
                self.history.push("MOV L, H".to_string());
            }
            0x6d => {
                self.l = self.l;
                self.history.push("MOV L, L".to_string());
            }
            0x6e => {
                self.l = self.memory[self.hl() as usize];
                self.history.push("MOV L, M".to_string());
            }
            0x6f => {
                self.l = self.a;
                self.history.push("MOV L, A".to_string());
            }
            0x70 => {
                self.memory[self.hl() as usize] = self.b;
                self.history.push("MOV M, B".to_string());
            }
            0x71 => {
                self.memory[self.hl() as usize] = self.c;
                self.history.push("MOV M, C".to_string());
            }
            0x72 => {
                self.memory[self.hl() as usize] = self.d;
                self.history.push("MOV M, D".to_string());
            }
            0x73 => {
                self.memory[self.hl() as usize] = self.e;
                self.history.push("MOV M, E".to_string());
            }
            0x74 => {
                self.memory[self.hl() as usize] = self.h;
                self.history.push("MOV M, H".to_string());
            }
            0x75 => {
                self.memory[self.hl() as usize] = self.l;
                self.history.push("MOV M, L".to_string());
            }
            0x76 => {
                self.halt = true;
                self.history.push("HLT".to_string());
            }
            0x77 => {
                self.memory[self.hl() as usize] = self.a;
                self.history.push("MOV M, A".to_string());
            }
            0x78 => {
                self.a = self.b;
                self.history.push("MOV A, B".to_string());
            }
            0x79 => {
                self.a = self.c;
                self.history.push("MOV A, C".to_string());
            }
            0x7a => {
                self.a = self.d;
                self.history.push("MOV A, D".to_string());
            }
            0x7b => {
                self.a = self.e;
                self.history.push("MOV A, E".to_string());
            }
            0x7c => {
                self.a = self.h;
                self.history.push("MOV A, H".to_string());
            }
            0x7d => {
                self.a = self.l;
                self.history.push("MOV A, L".to_string());
            }
            0x7e => {
                self.a = self.memory[self.hl() as usize];
                self.history.push("MOV A, M".to_string());
            }
            0x7f => {
                self.a = self.a;
                self.history.push("MOV A, A".to_string());
            }
            _ => {
                self.history
                    .push(format!("Unimplemented opcode: {:#04x}", self.read(self.pc)));
            }
        }
        self.pc += 1;
    }
}

// #[derive(Debug)]
// enum Reg {
//     B,
//     C,
//     D,
//     E,
//     H,
//     L,
//     M,
//     A,
// }
//
// #[derive(Debug)]
// enum Instruction {
//     Unimplemented(String),
//     Invalid,
//     Nop,
//     Lxi { reg: Reg, lo: u8, hi: u8 },
//     Stax { reg: Reg },
//     Inx { reg: Reg },
//     Inr { reg: Reg },
//     Dcr { reg: Reg },
//     Mvi { reg: Reg, lo: u8 },
//     Rlc,
//     Dad { reg: Reg },
//     Ldax { reg: Reg },
//     Dcx { reg: Reg },
//     Rrc,
//     Ral,
//     Rar,
//     Shld { lo: u8, hi: u8 },
//     Daa,
//     Lhld { lo: u8, hi: u8 },
//     Cma,
//     Sta { lo: u8, hi: u8 },
//     Stc,
//     Lda { lo: u8, hi: u8 },
//     Cmc,
//     Mov { from: Reg, to: Reg },
//     Hlt,
//     Add { reg: Reg, overflow: bool },
//     Sub { reg: Reg },
//     Sbb { reg: Reg },
//     Ana { reg: Reg },
//     Xra { reg: Reg },
//     Ora { reg: Reg },
//     Cmp { reg: Reg },
//     Rnz,
//     Pop { reg: Reg },
//     Jnz { lo: u8, hi: u8 },
//     Jmp { lo: u8, hi: u8 },
//     Cnz { lo: u8, hi: u8 },
//     Push { reg: Reg },
//     Adi { lo: u8 },
//     Rst { offset: u8 },
//     Rz,
//     Ret,
//     Jz { lo: u8, hi: u8 },
//     Cz { lo: u8, hi: u8 },
//     Call { lo: u8, hi: u8 },
//     Aci { lo: u8 },
//     Rnc,
//     Jnc { lo: u8, hi: u8 },
//     Out { lo: u8 },
//     Cnc { lo: u8, hi: u8 },
//     Sui { lo: u8 },
//     Rc,
//     Jc { lo: u8, hi: u8 },
//     In { lo: u8 },
//     Cc { lo: u8, hi: u8 },
//     Sbi { lo: u8 },
//     Rpo,
//     Jpo { lo: u8, hi: u8 },
//     Xthl,
//     Cpo { lo: u8, hi: u8 },
//     Ani { lo: u8 },
//     Rpe,
//     Pchl,
//     Jpe { lo: u8, hi: u8 },
//     Xchg,
//     Cpe { lo: u8, hi: u8 },
//     Xri { lo: u8 },
//     Rp,
//     Jp { lo: u8, hi: u8 },
//     Di,
//     Cp { lo: u8, hi: u8 },
//     Ori { lo: u8 },
//     Rm,
//     Sphl,
//     Jm { lo: u8, hi: u8 },
//     Ei,
//     Cm { lo: u8, hi: u8 },
//     Cpi { lo: u8 },
// }
//
// macro_rules! one_reg {
//     ($variant:ident, $reg:expr, $rom:expr, $pc:expr) => {{
//         $pc += 1;
//         Instruction::$variant {
//             reg: $reg,
//             lo: $rom[$pc],
//         }
//     }};
// }
//
// macro_rules! one {
//     ($variant:ident, $rom:expr, $pc:expr) => {{
//         $pc += 1;
//         Instruction::$variant { lo: $rom[$pc] }
//     }};
// }
//
// macro_rules! two_reg {
//     ($variant:ident, $reg:expr, $rom:expr, $pc:expr) => {{
//         $pc += 2;
//         Instruction::$variant {
//             reg: $reg,
//             lo: $rom[$pc - 1],
//             hi: $rom[$pc],
//         }
//     }};
// }
//
// macro_rules! two {
//     ($variant:ident, $rom:expr, $pc:expr) => {{
//         $pc += 2;
//         Instruction::$variant {
//             lo: $rom[$pc - 1],
//             hi: $rom[$pc],
//         }
//     }};
// }
//
// macro_rules! mov {
//     ($from:expr, $to:expr) => {
//         Instruction::Mov {
//             from: $from,
//             to: $to,
//         }
//     };
// }
//
// macro_rules! add {
//     ($reg:expr, $overflow:expr) => {
//         Instruction::Add {
//             reg: $reg,
//             overflow: $overflow,
//         }
//     };
// }
//
// fn dissemble8080(rom: &[u8]) -> Vec<Instruction> {
//     let mut instructions = Vec::new();
//
//     let mut pc = 0;
//     while pc < rom.len() {
//         let instruction = match rom[pc] {
//             0x00 => Instruction::Nop,
//             0x01 => two_reg!(Lxi, Reg::B, rom, pc),
//             0x02 => Instruction::Stax { reg: Reg::B },
//             0x03 => Instruction::Inx { reg: Reg::B },
//             0x04 => Instruction::Inr { reg: Reg::B },
//             0x05 => Instruction::Dcr { reg: Reg::B },
//             0x06 => one_reg!(Mvi, Reg::B, rom, pc),
//             0x07 => Instruction::Rlc,
//             0x08 => Instruction::Invalid,
//             0x09 => Instruction::Dad { reg: Reg::B },
//             0x0a => Instruction::Ldax { reg: Reg::B },
//             0x0b => Instruction::Dcx { reg: Reg::B },
//             0x0c => Instruction::Inr { reg: Reg::C },
//             0x0d => Instruction::Dcr { reg: Reg::C },
//             0x0e => one_reg!(Mvi, Reg::C, rom, pc),
//             0x0f => Instruction::Rrc,
//             0x10 => Instruction::Invalid,
//             0x11 => two_reg!(Lxi, Reg::D, rom, pc),
//             0x12 => Instruction::Stax { reg: Reg::D },
//             0x13 => Instruction::Inx { reg: Reg::D },
//             0x14 => Instruction::Inr { reg: Reg::D },
//             0x15 => Instruction::Dcr { reg: Reg::D },
//             0x16 => one_reg!(Mvi, Reg::D, rom, pc),
//             0x17 => Instruction::Ral,
//             0x18 => Instruction::Invalid,
//             0x19 => Instruction::Dad { reg: Reg::D },
//             0x1a => Instruction::Ldax { reg: Reg::D },
//             0x1b => Instruction::Dcx { reg: Reg::D },
//             0x1c => Instruction::Inr { reg: Reg::E },
//             0x1d => Instruction::Dcr { reg: Reg::E },
//             0x1e => one_reg!(Mvi, Reg::E, rom, pc),
//             0x1f => Instruction::Rar,
//             0x20 => Instruction::Invalid,
//             0x21 => two_reg!(Lxi, Reg::H, rom, pc),
//             0x22 => two!(Shld, rom, pc),
//             0x23 => Instruction::Inx { reg: Reg::H },
//             0x24 => Instruction::Inr { reg: Reg::H },
//             0x25 => Instruction::Dcr { reg: Reg::H },
//             0x26 => one_reg!(Mvi, Reg::H, rom, pc),
//             0x27 => Instruction::Daa,
//             0x28 => Instruction::Invalid,
//             0x29 => Instruction::Dad { reg: Reg::H },
//             0x2a => two!(Lhld, rom, pc),
//             0x2b => Instruction::Dcx { reg: Reg::H },
//             0x2c => Instruction::Inr { reg: Reg::L },
//             0x2d => Instruction::Dcr { reg: Reg::L },
//             0x2e => one_reg!(Mvi, Reg::L, rom, pc),
//             0x2f => Instruction::Cma,
//             0x30 => Instruction::Invalid,
//             0x31 => two_reg!(Lxi, Reg::M, rom, pc),
//             0x32 => two!(Sta, rom, pc),
//             0x33 => Instruction::Inx { reg: Reg::M },
//             0x34 => Instruction::Inr { reg: Reg::M },
//             0x35 => Instruction::Dcr { reg: Reg::M },
//             0x36 => one_reg!(Mvi, Reg::M, rom, pc),
//             0x37 => Instruction::Stc,
//             0x38 => Instruction::Invalid,
//             0x39 => Instruction::Dad { reg: Reg::M },
//             0x3a => two!(Lda, rom, pc),
//             0x3b => Instruction::Dcx { reg: Reg::M },
//             0x3c => Instruction::Inr { reg: Reg::A },
//             0x3d => Instruction::Dcr { reg: Reg::A },
//             0x3e => one_reg!(Mvi, Reg::A, rom, pc),
//             0x3f => Instruction::Cmc,
//             0x40 => mov!(Reg::B, Reg::B),
//             0x41 => mov!(Reg::C, Reg::B),
//             0x42 => mov!(Reg::D, Reg::B),
//             0x43 => mov!(Reg::E, Reg::B),
//             0x44 => mov!(Reg::H, Reg::B),
//             0x45 => mov!(Reg::L, Reg::B),
//             0x46 => mov!(Reg::M, Reg::B),
//             0x47 => mov!(Reg::A, Reg::B),
//             0x48 => mov!(Reg::B, Reg::C),
//             0x49 => mov!(Reg::C, Reg::C),
//             0x4a => mov!(Reg::D, Reg::C),
//             0x4b => mov!(Reg::E, Reg::C),
//             0x4c => mov!(Reg::H, Reg::C),
//             0x4d => mov!(Reg::L, Reg::C),
//             0x4e => mov!(Reg::M, Reg::C),
//             0x4f => mov!(Reg::A, Reg::C),
//             0x50 => mov!(Reg::B, Reg::D),
//             0x51 => mov!(Reg::C, Reg::D),
//             0x52 => mov!(Reg::D, Reg::D),
//             0x53 => mov!(Reg::E, Reg::D),
//             0x54 => mov!(Reg::H, Reg::D),
//             0x55 => mov!(Reg::L, Reg::D),
//             0x56 => mov!(Reg::M, Reg::D),
//             0x57 => mov!(Reg::A, Reg::D),
//             0x58 => mov!(Reg::B, Reg::E),
//             0x59 => mov!(Reg::C, Reg::E),
//             0x5a => mov!(Reg::D, Reg::E),
//             0x5b => mov!(Reg::E, Reg::E),
//             0x5c => mov!(Reg::H, Reg::E),
//             0x5d => mov!(Reg::L, Reg::E),
//             0x5e => mov!(Reg::M, Reg::E),
//             0x5f => mov!(Reg::A, Reg::E),
//             0x60 => mov!(Reg::B, Reg::H),
//             0x61 => mov!(Reg::C, Reg::H),
//             0x62 => mov!(Reg::D, Reg::H),
//             0x63 => mov!(Reg::E, Reg::H),
//             0x64 => mov!(Reg::H, Reg::H),
//             0x65 => mov!(Reg::L, Reg::H),
//             0x66 => mov!(Reg::M, Reg::H),
//             0x67 => mov!(Reg::A, Reg::H),
//             0x68 => mov!(Reg::B, Reg::L),
//             0x69 => mov!(Reg::C, Reg::L),
//             0x6a => mov!(Reg::D, Reg::L),
//             0x6b => mov!(Reg::E, Reg::L),
//             0x6c => mov!(Reg::H, Reg::L),
//             0x6d => mov!(Reg::L, Reg::L),
//             0x6e => mov!(Reg::M, Reg::L),
//             0x6f => mov!(Reg::A, Reg::L),
//             0x70 => mov!(Reg::B, Reg::M),
//             0x71 => mov!(Reg::C, Reg::M),
//             0x72 => mov!(Reg::D, Reg::M),
//             0x73 => mov!(Reg::E, Reg::M),
//             0x74 => mov!(Reg::H, Reg::M),
//             0x75 => mov!(Reg::L, Reg::M),
//             0x76 => Instruction::Hlt,
//             0x77 => mov!(Reg::A, Reg::M),
//             0x78 => mov!(Reg::B, Reg::A),
//             0x79 => mov!(Reg::C, Reg::A),
//             0x7a => mov!(Reg::D, Reg::A),
//             0x7b => mov!(Reg::E, Reg::A),
//             0x7c => mov!(Reg::H, Reg::A),
//             0x7d => mov!(Reg::L, Reg::A),
//             0x7e => mov!(Reg::M, Reg::A),
//             0x7f => mov!(Reg::A, Reg::A),
//             0x80 => add!(Reg::B, false),
//             0x81 => add!(Reg::C, false),
//             0x82 => add!(Reg::D, false),
//             0x83 => add!(Reg::E, false),
//             0x84 => add!(Reg::H, false),
//             0x85 => add!(Reg::L, false),
//             0x86 => add!(Reg::M, false),
//             0x87 => add!(Reg::A, false),
//             0x88 => add!(Reg::B, true),
//             0x89 => add!(Reg::C, true),
//             0x8a => add!(Reg::D, true),
//             0x8b => add!(Reg::E, true),
//             0x8c => add!(Reg::H, true),
//             0x8d => add!(Reg::L, true),
//             0x8e => add!(Reg::M, true),
//             0x8f => add!(Reg::A, true),
//             0x90 => Instruction::Sub { reg: Reg::B },
//             0x91 => Instruction::Sub { reg: Reg::C },
//             0x92 => Instruction::Sub { reg: Reg::D },
//             0x93 => Instruction::Sub { reg: Reg::E },
//             0x94 => Instruction::Sub { reg: Reg::H },
//             0x95 => Instruction::Sub { reg: Reg::L },
//             0x96 => Instruction::Sub { reg: Reg::M },
//             0x97 => Instruction::Sub { reg: Reg::A },
//             0x98 => Instruction::Sbb { reg: Reg::B },
//             0x99 => Instruction::Sbb { reg: Reg::C },
//             0x9a => Instruction::Sbb { reg: Reg::D },
//             0x9b => Instruction::Sbb { reg: Reg::E },
//             0x9c => Instruction::Sbb { reg: Reg::H },
//             0x9d => Instruction::Sbb { reg: Reg::L },
//             0x9e => Instruction::Sbb { reg: Reg::M },
//             0x9f => Instruction::Sbb { reg: Reg::A },
//             0xa0 => Instruction::Ana { reg: Reg::B },
//             0xa1 => Instruction::Ana { reg: Reg::C },
//             0xa2 => Instruction::Ana { reg: Reg::D },
//             0xa3 => Instruction::Ana { reg: Reg::E },
//             0xa4 => Instruction::Ana { reg: Reg::H },
//             0xa5 => Instruction::Ana { reg: Reg::L },
//             0xa6 => Instruction::Ana { reg: Reg::M },
//             0xa7 => Instruction::Ana { reg: Reg::A },
//             0xa8 => Instruction::Xra { reg: Reg::B },
//             0xa9 => Instruction::Xra { reg: Reg::C },
//             0xaa => Instruction::Xra { reg: Reg::D },
//             0xab => Instruction::Xra { reg: Reg::E },
//             0xac => Instruction::Xra { reg: Reg::H },
//             0xad => Instruction::Xra { reg: Reg::L },
//             0xae => Instruction::Xra { reg: Reg::M },
//             0xaf => Instruction::Xra { reg: Reg::A },
//             0xb0 => Instruction::Ora { reg: Reg::B },
//             0xb1 => Instruction::Ora { reg: Reg::C },
//             0xb2 => Instruction::Ora { reg: Reg::D },
//             0xb3 => Instruction::Ora { reg: Reg::E },
//             0xb4 => Instruction::Ora { reg: Reg::H },
//             0xb5 => Instruction::Ora { reg: Reg::L },
//             0xb6 => Instruction::Ora { reg: Reg::M },
//             0xb7 => Instruction::Ora { reg: Reg::A },
//             0xb8 => Instruction::Cmp { reg: Reg::B },
//             0xb9 => Instruction::Cmp { reg: Reg::C },
//             0xba => Instruction::Cmp { reg: Reg::D },
//             0xbb => Instruction::Cmp { reg: Reg::E },
//             0xbc => Instruction::Cmp { reg: Reg::H },
//             0xbd => Instruction::Cmp { reg: Reg::L },
//             0xbe => Instruction::Cmp { reg: Reg::M },
//             0xbf => Instruction::Cmp { reg: Reg::A },
//             0xc0 => Instruction::Rnz,
//             0xc1 => Instruction::Pop { reg: Reg::B },
//             0xc2 => two!(Jnz, rom, pc),
//             0xc3 => two!(Jmp, rom, pc),
//             0xc4 => two!(Cnz, rom, pc),
//             0xc5 => Instruction::Push { reg: Reg::B },
//             0xc6 => one!(Adi, rom, pc),
//             0xc7 => Instruction::Rst { offset: 0x00 },
//             0xc8 => Instruction::Rz,
//             0xc9 => Instruction::Ret,
//             0xca => two!(Jz, rom, pc),
//             0xcb => Instruction::Invalid,
//             0xcc => two!(Cz, rom, pc),
//             0xcd => two!(Call, rom, pc),
//             0xce => one!(Aci, rom, pc),
//             0xcf => Instruction::Rst { offset: 0x08 },
//             0xd0 => Instruction::Rnc,
//             0xd1 => Instruction::Pop { reg: Reg::D },
//             0xd2 => two!(Jnc, rom, pc),
//             0xd3 => one!(Out, rom, pc),
//             0xd4 => two!(Cnc, rom, pc),
//             0xd5 => Instruction::Push { reg: Reg::D },
//             0xd6 => one!(Sui, rom, pc),
//             0xd7 => Instruction::Rst { offset: 0x10 },
//             0xd8 => Instruction::Rc,
//             0xd9 => Instruction::Invalid,
//             0xda => two!(Jc, rom, pc),
//             0xdb => one!(In, rom, pc),
//             0xdc => two!(Cc, rom, pc),
//             0xdd => Instruction::Invalid,
//             0xde => one!(Sbi, rom, pc),
//             0xdf => Instruction::Rst { offset: 0x18 },
//             0xe0 => Instruction::Rpo,
//             0xe1 => Instruction::Pop { reg: Reg::H },
//             0xe2 => two!(Jpo, rom, pc),
//             0xe3 => Instruction::Xthl,
//             0xe4 => two!(Cpo, rom, pc),
//             0xe5 => Instruction::Push { reg: Reg::H },
//             0xe6 => one!(Ani, rom, pc),
//             0xe7 => Instruction::Rst { offset: 0x20 },
//             0xe8 => Instruction::Rpe,
//             0xe9 => Instruction::Pchl,
//             0xea => two!(Jpe, rom, pc),
//             0xeb => Instruction::Xchg,
//             0xec => two!(Cpe, rom, pc),
//             0xed => Instruction::Invalid,
//             0xee => one!(Xri, rom, pc),
//             0xef => Instruction::Rst { offset: 0x28 },
//             0xf0 => Instruction::Rp,
//             0xf1 => Instruction::Pop { reg: Reg::A },
//             0xf2 => two!(Jp, rom, pc),
//             0xf3 => Instruction::Di,
//             0xf4 => two!(Cp, rom, pc),
//             0xf5 => Instruction::Push { reg: Reg::A },
//             0xf6 => one!(Ori, rom, pc),
//             0xf7 => Instruction::Rst { offset: 0x30 },
//             0xf8 => Instruction::Rm,
//             0xf9 => Instruction::Sphl,
//             0xfa => two!(Jm, rom, pc),
//             0xfb => Instruction::Ei,
//             0xfc => two!(Cm, rom, pc),
//             0xfd => Instruction::Invalid,
//             0xfe => one!(Cpi, rom, pc),
//             0xff => Instruction::Rst { offset: 0x38 },
//             // op => Instruction::Unimplemented(format!("{op:x}")),
//         };
//         pc += 1;
//
//         instructions.push(instruction);
//     }
//
//     instructions
// }
