#![allow(unused)]
use std::io::{self, BufRead, Read};

use anyhow::Result;

use macroquad::prelude::*;

const PIXEL_SIZE: i32 = 3;
const WIDTH: i32 = 224 * PIXEL_SIZE;
const HEIGHT: i32 = 256 * PIXEL_SIZE;

fn window_conf() -> Conf {
    Conf {
        window_title: "8080 Emulator".to_owned(),
        fullscreen: false,
        window_resizable: false,
        window_width: WIDTH,
        window_height: HEIGHT,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<()> {
    println!("8080 emulator");

    let rom = std::fs::read("./rom/space-invaders/invaders").expect("Unable to read file");

    let mut cpu = Cpu8080::new();
    cpu.load(&rom);
    // cpu.mirror = 0x400;
    //
    // for _ in 0..40_500 {
    //     let pc = cpu.pc;
    //     cpu.step();
    //     println!("{:#06x} {:?}", pc, cpu.history.last().unwrap());
    // }
    //
    // dbg!(
    //     cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.pc, cpu.sp, cpu.cy, cpu.p, cpu.ac,
    //     cpu.z, cpu.s
    // );
    //
    // let stdin = io::stdin();
    // loop {
    //     let mut buffer = String::new();
    //     stdin.lock().read_line(&mut buffer)?;
    //     if buffer.as_str() == "q\n" {
    //         break;
    //     }
    //
    //     if buffer.as_str() == "d\n" {
    //         dbg!(
    //             cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.pc, cpu.sp, cpu.cy, cpu.p,
    //             cpu.ac, cpu.z, cpu.s
    //         );
    //         continue;
    //     }
    //
    //     let pc = cpu.pc;
    //     cpu.step();
    //     println!("{:#06x} {:?}", pc, cpu.history.last().unwrap());
    // }
    // return Ok(());

    loop {
        let delta = get_frame_time();

        for i in 0..(2_000_000. * delta) as usize {
            let pc = cpu.pc;
            cpu.step();
            println!("{:#06x} {:?}", pc, cpu.history.last().unwrap());
        }

        clear_background(BLACK);

        // for space invader, the vram starts from 0x2400 until 0x3fff
        // the color is monocrome so i need to bitshift to get 8 pixel
        for mem_pointer in 0x2400..0x4000 {
            // draw 8 pixel at a time
            for offset in 0..8 {
                let color = match (cpu.memory[mem_pointer]) & (1 << offset) > 0 {
                    true => WHITE,
                    _ => BLACK,
                };

                let x =
                    ((((mem_pointer - 0x2400) * 8 + offset) % 256) * PIXEL_SIZE as usize) as f32;
                let y =
                    ((((mem_pointer - 0x2400) * 8 + offset) / 256) * PIXEL_SIZE as usize) as f32;

                let w = PIXEL_SIZE as f32;
                let h = PIXEL_SIZE as f32;

                let x_center = WIDTH as f32 / 2.;
                let y_center = HEIGHT as f32 / 2.;

                // -90 deg rotation since the memory buffer is rotated
                let rot_x = (y - x_center) + x_center;
                let rot_y = -(x - y_center) + y_center;

                draw_rectangle(rot_x, rot_y, w, h, color)
            }
        }

        next_frame().await;
    }

    // for i in 0..0x4000 / 0x10 {
    //     print!("{:#06x}  ", i * 0x10);
    //     for mem in cpu.memory.iter().skip(i * 0x10).take(0x10) {
    //         print!("{:#04x} ", mem);
    //     }
    //     println!();
    // }
    //
    // dbg!(
    //     cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.pc, cpu.sp, cpu.cy, cpu.p, cpu.ac,
    //     cpu.z, cpu.s
    // );
    //
    // for _ in 0..2 {
    //     let pc = cpu.pc;
    //     cpu.step();
    //     println!("{:#06x} {:?}", pc, cpu.history.last().unwrap());
    // }
    //
    // dbg!(
    //     cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.pc, cpu.sp, cpu.cy, cpu.p, cpu.ac,
    //     cpu.z, cpu.s
    // );

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

    pub interrupt: bool,

    pub halt: bool,

    pub memory: [u8; 0x10000],
    /// special for space invaders
    pub mirror: u16,

    pub history: Vec<String>,
}

macro_rules! flag {
    ($self:ident, $reg:expr) => {
        $self.z = $reg == 0;
        $self.s = $reg & (1 << 7) != 0;
        $self.p = $reg.count_ones() % 2 == 0;
        $self.ac = $reg & 0x0f > 9;
    };
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
            interrupt: false,
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

    fn pop(&mut self) -> u16 {
        let value = self.read(self.sp + 1) as u16 | (self.read(self.sp) as u16) << 8;
        self.sp += 2;
        value
    }

    fn push(&mut self, value: u16) {
        self.sp -= 2;
        self.memory[self.sp as usize] = (value >> 8) as u8;
        self.memory[(self.sp + 1) as usize] = value as u8;
    }

    fn call(&mut self, addr: u16) {
        self.sp -= 2;
        self.memory[self.sp as usize] = (self.pc >> 8) as u8;
        self.memory[(self.sp + 1) as usize] = self.pc as u8;
        self.pc = addr.wrapping_sub(1);
    }

    fn step(&mut self) {
        match self.read(self.pc) {
            0x00 => self.history.push("NOP".to_string()),
            0x01 => {
                let addr = self.next_memory();
                self.set_bc(addr);
                self.pc = self.pc.wrapping_add(2);
                self.history.push(format!("LXI B, {:#06x}", addr));
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
                flag!(self, self.b);
                self.history.push("INR B".to_string());
            }
            0x05 => {
                self.b = self.b.wrapping_sub(1);
                flag!(self, self.b);
                self.history.push("DCR B".to_string());
            }
            0x06 => {
                self.b = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                flag!(self, self.c);
                self.history.push("INR C".to_string());
            }
            0x0d => {
                self.c = self.c.wrapping_sub(1);
                flag!(self, self.c);
                self.history.push("DCR C".to_string());
            }
            0x0e => {
                self.c = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                let addr = self.next_memory();
                self.set_de(addr);
                self.pc = self.pc.wrapping_add(2);
                self.history.push(format!("LXI D, {:#06x}", addr));
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
                flag!(self, self.d);
                self.history.push("INR D".to_string());
            }
            0x15 => {
                self.d = self.d.wrapping_sub(1);
                flag!(self, self.d);
                self.history.push("DCR D".to_string());
            }
            0x16 => {
                self.d = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                flag!(self, self.e);
                self.history.push("INR E".to_string());
            }
            0x1d => {
                self.e = self.e.wrapping_sub(1);
                flag!(self, self.e);
                self.history.push("DCR E".to_string());
            }
            0x1e => {
                self.e = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                let addr = self.next_memory();
                self.set_hl(addr);
                self.pc = self.pc.wrapping_add(2);
                self.history.push(format!("LXI H, {:#06x}", addr));
            }
            0x22 => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
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
                flag!(self, self.h);
                self.history.push("INR H".to_string());
            }
            0x25 => {
                self.h = self.h.wrapping_sub(1);
                flag!(self, self.h);
                self.history.push("DCR H".to_string());
            }
            0x26 => {
                self.h = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                self.pc = self.pc.wrapping_add(2);
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
                flag!(self, self.l);
                self.history.push("INR L".to_string());
            }
            0x2d => {
                self.l = self.l.wrapping_sub(1);
                flag!(self, self.l);
                self.history.push("DCR L".to_string());
            }
            0x2e => {
                self.l = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
                self.pc = self.pc.wrapping_add(2);
                self.history.push(format!("LXI SP, {:#06x}", self.sp));
            }
            0x32 => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
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
                self.pc = self.pc.wrapping_add(1);
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
                self.pc = self.pc.wrapping_add(2);
                self.a = self.memory[addr as usize];
                self.history.push(format!("LDA {:#06x}", addr));
            }
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                self.history.push("DCX SP".to_string());
            }
            0x3c => {
                self.a = self.a.wrapping_add(1);
                flag!(self, self.a);
                self.history.push("INR A".to_string());
            }
            0x3d => {
                self.a = self.a.wrapping_sub(1);
                flag!(self, self.a);
                self.history.push("DCR A".to_string());
            }
            0x3e => {
                self.a = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
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
            0x80 => {
                (self.a, self.cy) = self.a.overflowing_add(self.b);
                flag!(self, self.a);
                self.history.push("ADD B".to_string());
            }
            0x81 => {
                (self.a, self.cy) = self.a.overflowing_add(self.c);
                flag!(self, self.a);
                self.history.push("ADD C".to_string());
            }
            0x82 => {
                (self.a, self.cy) = self.a.overflowing_add(self.d);
                flag!(self, self.a);
                self.history.push("ADD D".to_string());
            }
            0x83 => {
                (self.a, self.cy) = self.a.overflowing_add(self.e);
                flag!(self, self.a);
                self.history.push("ADD E".to_string());
            }
            0x84 => {
                (self.a, self.cy) = self.a.overflowing_add(self.h);
                flag!(self, self.a);
                self.history.push("ADD H".to_string());
            }
            0x85 => {
                (self.a, self.cy) = self.a.overflowing_add(self.l);
                flag!(self, self.a);
                self.history.push("ADD L".to_string());
            }
            0x86 => {
                let value = self.memory[self.hl() as usize];
                (self.a, self.cy) = self.a.overflowing_add(value);
                flag!(self, self.a);
                self.history.push("ADD M".to_string());
            }
            0x87 => {
                (self.a, self.cy) = self.a.overflowing_add(self.a);
                flag!(self, self.a);
                self.history.push("ADD A".to_string());
            }
            0x88 => {
                (self.a, self.cy) = self.a.overflowing_add(self.b.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC B".to_string());
            }
            0x89 => {
                (self.a, self.cy) = self.a.overflowing_add(self.c.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC C".to_string());
            }
            0x8a => {
                (self.a, self.cy) = self.a.overflowing_add(self.d.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC D".to_string());
            }
            0x8b => {
                (self.a, self.cy) = self.a.overflowing_add(self.e.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC E".to_string());
            }
            0x8c => {
                (self.a, self.cy) = self.a.overflowing_add(self.h.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC H".to_string());
            }
            0x8d => {
                (self.a, self.cy) = self.a.overflowing_add(self.l.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC L".to_string());
            }
            0x8e => {
                let value = self.memory[self.hl() as usize];
                (self.a, self.cy) = self.a.overflowing_add(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC M".to_string());
            }
            0x8f => {
                (self.a, self.cy) = self.a.overflowing_add(self.a.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("ADC A".to_string());
            }
            0x90 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.b);
                flag!(self, self.a);
                self.history.push("SUB B".to_string());
            }
            0x91 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.c);
                flag!(self, self.a);
                self.history.push("SUB C".to_string());
            }
            0x92 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.d);
                flag!(self, self.a);
                self.history.push("SUB D".to_string());
            }
            0x93 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.e);
                flag!(self, self.a);
                self.history.push("SUB E".to_string());
            }
            0x94 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.h);
                flag!(self, self.a);
                self.history.push("SUB H".to_string());
            }
            0x95 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.l);
                flag!(self, self.a);
                self.history.push("SUB L".to_string());
            }
            0x96 => {
                let value = self.memory[self.hl() as usize];
                (self.a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, self.a);
                self.history.push("SUB M".to_string());
            }
            0x97 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.a);
                flag!(self, self.a);
                self.history.push("SUB A".to_string());
            }
            0x98 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.b.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB B".to_string());
            }
            0x99 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.c.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB C".to_string());
            }
            0x9a => {
                (self.a, self.cy) = self.a.overflowing_sub(self.d.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB D".to_string());
            }
            0x9b => {
                (self.a, self.cy) = self.a.overflowing_sub(self.e.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB E".to_string());
            }
            0x9c => {
                (self.a, self.cy) = self.a.overflowing_sub(self.h.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB H".to_string());
            }
            0x9d => {
                (self.a, self.cy) = self.a.overflowing_sub(self.l.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB L".to_string());
            }
            0x9e => {
                let value = self.memory[self.hl() as usize];
                (self.a, self.cy) = self.a.overflowing_sub(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB M".to_string());
            }
            0x9f => {
                (self.a, self.cy) = self.a.overflowing_sub(self.a.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.history.push("SBB A".to_string());
            }
            0xa0 => {
                self.a &= self.b;
                flag!(self, self.a);
                self.history.push("ANA B".to_string());
            }
            0xa1 => {
                self.a &= self.c;
                flag!(self, self.a);
                self.history.push("ANA C".to_string());
            }
            0xa2 => {
                self.a &= self.d;
                flag!(self, self.a);
                self.history.push("ANA D".to_string());
            }
            0xa3 => {
                self.a &= self.e;
                flag!(self, self.a);
                self.history.push("ANA E".to_string());
            }
            0xa4 => {
                self.a &= self.h;
                flag!(self, self.a);
                self.history.push("ANA H".to_string());
            }
            0xa5 => {
                self.a &= self.l;
                flag!(self, self.a);
                self.history.push("ANA L".to_string());
            }
            0xa6 => {
                let value = self.memory[self.hl() as usize];
                self.a &= value;
                flag!(self, self.a);
                self.history.push("ANA M".to_string());
            }
            0xa7 => {
                self.a &= self.a;
                flag!(self, self.a);
                self.history.push("ANA A".to_string());
            }
            0xa8 => {
                self.a ^= self.b;
                flag!(self, self.a);
                self.history.push("XRA B".to_string());
            }
            0xa9 => {
                self.a ^= self.c;
                flag!(self, self.a);
                self.history.push("XRA C".to_string());
            }
            0xaa => {
                self.a ^= self.d;
                flag!(self, self.a);
                self.history.push("XRA D".to_string());
            }
            0xab => {
                self.a ^= self.e;
                flag!(self, self.a);
                self.history.push("XRA E".to_string());
            }
            0xac => {
                self.a ^= self.h;
                flag!(self, self.a);
                self.history.push("XRA H".to_string());
            }
            0xad => {
                self.a ^= self.l;
                flag!(self, self.a);
                self.history.push("XRA L".to_string());
            }
            0xae => {
                let value = self.memory[self.hl() as usize];
                self.a ^= value;
                flag!(self, self.a);
                self.history.push("XRA M".to_string());
            }
            0xaf => {
                self.a ^= self.a;
                flag!(self, self.a);
                self.history.push("XRA A".to_string());
            }
            0xb0 => {
                self.a |= self.b;
                flag!(self, self.a);
                self.history.push("ORA B".to_string());
            }
            0xb1 => {
                self.a |= self.c;
                flag!(self, self.a);
                self.history.push("ORA C".to_string());
            }
            0xb2 => {
                self.a |= self.d;
                flag!(self, self.a);
                self.history.push("ORA D".to_string());
            }
            0xb3 => {
                self.a |= self.e;
                flag!(self, self.a);
                self.history.push("ORA E".to_string());
            }
            0xb4 => {
                self.a |= self.h;
                flag!(self, self.a);
                self.history.push("ORA H".to_string());
            }
            0xb5 => {
                self.a |= self.l;
                flag!(self, self.a);
                self.history.push("ORA L".to_string());
            }
            0xb6 => {
                let value = self.memory[self.hl() as usize];
                self.a |= value;
                flag!(self, self.a);
                self.history.push("ORA M".to_string());
            }
            0xb7 => {
                self.a |= self.a;
                flag!(self, self.a);
                self.history.push("ORA A".to_string());
            }
            0xb8 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.b);
                flag!(self, self.a);
                self.history.push("CMP B".to_string());
            }
            0xb9 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.c);
                flag!(self, self.a);
                self.history.push("CMP C".to_string());
            }
            0xba => {
                (self.a, self.cy) = self.a.overflowing_sub(self.d);
                flag!(self, self.a);
                self.history.push("CMP D".to_string());
            }
            0xbb => {
                (self.a, self.cy) = self.a.overflowing_sub(self.e);
                flag!(self, self.a);
                self.history.push("CMP E".to_string());
            }
            0xbc => {
                (self.a, self.cy) = self.a.overflowing_sub(self.h);
                flag!(self, self.a);
                self.history.push("CMP H".to_string());
            }
            0xbd => {
                (self.a, self.cy) = self.a.overflowing_sub(self.l);
                flag!(self, self.a);
                self.history.push("CMP L".to_string());
            }
            0xbe => {
                let value = self.memory[self.hl() as usize];
                (self.a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, self.a);
                self.history.push("CMP M".to_string());
            }
            0xbf => {
                (self.a, self.cy) = self.a.overflowing_sub(self.a);
                flag!(self, self.a);
                self.history.push("CMP A".to_string());
            }
            0xc0 => {
                if !self.z {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RNZ".to_string());
            }
            0xc1 => {
                let bc = self.pop();
                self.set_bc(bc);
                self.history.push("POP B".to_string());
            }
            0xc2 => {
                let addr = self.next_memory();
                self.pc = match self.z {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JNZ {:#06x}", addr));
            }
            0xc3 => {
                let addr = self.next_memory();
                self.pc = addr.wrapping_sub(1);
                self.history.push(format!("JMP {:#06x}", addr));
            }
            0xc4 => {
                let addr = self.next_memory();
                if !self.z {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CNZ {:#06x}", addr));
            }
            0xc5 => {
                self.push(self.bc());
                self.history.push("PUSH B".to_string());
            }
            0xc6 => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_add(value);
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("ADI {:#04x}", value));
            }
            0xc7 => {
                self.call(0x00);
                self.history.push("RST 0".to_string());
            }
            0xc8 => {
                if self.z {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RZ".to_string());
            }
            0xc9 => {
                self.pc = self.pop().wrapping_add(2);
                self.history.push("RET".to_string());
            }
            0xca => {
                let addr = self.next_memory();
                self.pc = match self.z {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JZ {:#06x}", addr));
            }
            0xcb => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xcc => {
                let addr = self.next_memory();
                if self.z {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CZ {:#06x}", addr));
            }
            0xcd => {
                let addr = self.next_memory();
                self.call(addr);
                self.history.push(format!("CALL {:#06x}", addr));
            }
            0xce => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_add(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("ACI {:#04x}", value));
            }
            0xcf => {
                self.call(0x08);
                self.history.push("RST 1".to_string());
            }
            0xd0 => {
                if !self.cy {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RNC".to_string());
            }
            0xd1 => {
                let de = self.pop();
                self.set_de(de);
                self.history.push("POP D".to_string());
            }
            0xd2 => {
                let addr = self.next_memory();
                self.pc = match self.cy {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JNC {:#06x}", addr));
            }
            0xd3 => {
                let port = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("OUT {:#04x}", port));
            }
            0xd4 => {
                let addr = self.next_memory();
                if !self.cy {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CNC {:#06x}", addr));
            }
            0xd5 => {
                self.push(self.de());
                self.history.push("PUSH D".to_string());
            }
            0xd6 => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("SUI {:#04x}", value));
            }
            0xd7 => {
                self.call(0x10);
                self.history.push("RST 2".to_string());
            }
            0xd8 => {
                if self.cy {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RC".to_string());
            }
            0xd9 => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xda => {
                let addr = self.next_memory();
                self.pc = match self.cy {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JC {:#06x}", addr));
            }
            0xdb => {
                let port = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("IN {:#04x}", port));
            }
            0xdc => {
                let addr = self.next_memory();
                if self.cy {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CC {:#06x}", addr));
            }
            0xdd => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xde => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_sub(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("SBI {:#04x}", value));
            }
            0xdf => {
                self.call(0x18);
                self.history.push("RST 3".to_string());
            }
            0xe0 => {
                if !self.p {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RPO".to_string());
            }
            0xe1 => {
                let hl = self.pop();
                self.set_hl(hl);
                self.history.push("POP H".to_string());
            }
            0xe2 => {
                let addr = self.next_memory();
                self.pc = match self.p {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JPO {:#06x}", addr));
            }
            0xe3 => {
                let hl = self.pop();
                self.push(self.hl());
                self.set_hl(hl);
                self.history.push("XTHL".to_string());
            }
            0xe4 => {
                let addr = self.next_memory();
                if !self.p {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CPO {:#06x}", addr));
            }
            0xe5 => {
                self.push(self.hl());
                self.history.push("PUSH H".to_string());
            }
            0xe6 => {
                let value = self.read(self.pc + 1);
                self.a &= value;
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("ANI {:#04x}", value));
            }
            0xe7 => {
                self.call(0x20);
                self.history.push("RST 4".to_string());
            }
            0xe8 => {
                if self.p {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RPE".to_string());
            }
            0xe9 => {
                self.pc = self.hl();
                self.history.push("PCHL".to_string());
            }
            0xea => {
                let addr = self.next_memory();
                self.pc = match self.p {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JPE {:#06x}", addr));
            }
            0xeb => {
                let de = self.de();
                self.set_de(self.hl());
                self.set_hl(de);
                self.history.push("XCHG".to_string());
            }
            0xec => {
                let addr = self.next_memory();
                if self.p {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CPE {:#06x}", addr));
            }
            0xed => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xee => {
                let value = self.read(self.pc + 1);
                self.a ^= value;
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("XRI {:#04x}", value));
            }
            0xef => {
                self.call(0x28);
                self.history.push("RST 5".to_string());
            }
            0xf0 => {
                if !self.s {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RP".to_string());
            }
            0xf1 => {
                let value = self.pop();
                self.s = value & (1 << 7) != 0;
                self.z = value & (1 << 6) != 0;
                self.ac = value & (1 << 4) != 0;
                self.p = value & (1 << 2) != 0;
                self.cy = value & 1 != 0;
                self.history.push("POP PSW".to_string());
            }
            0xf2 => {
                let addr = self.next_memory();
                self.pc = match self.s {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JP {:#06x}", addr));
            }
            0xf3 => {
                self.interrupt = false;
                self.history.push("DI".to_string());
            }
            0xf4 => {
                let addr = self.next_memory();
                if !self.s {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CP {:#06x}", addr));
            }
            0xf5 => {
                let mut addr = self.a as u16;
                addr |= (self.s as u16) << 7;
                addr |= (self.z as u16) << 6;
                addr |= (self.ac as u16) << 4;
                addr |= (self.p as u16) << 2;
                addr |= self.cy as u16;
                self.push(addr);

                self.history.push("PUSH PSW".to_string());
            }
            0xf6 => {
                let value = self.read(self.pc + 1);
                self.a |= value;
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("ORI {:#04x}", value));
            }
            0xf7 => {
                self.call(0x30);
                self.history.push("RST 6".to_string());
            }
            0xf8 => {
                if self.s {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.history.push("RM".to_string());
            }
            0xf9 => {
                self.sp = self.hl();
                self.history.push("SPHL".to_string());
            }
            0xfa => {
                let addr = self.next_memory();
                self.pc = match self.s {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.history.push(format!("JM {:#06x}", addr));
            }
            0xfb => {
                self.interrupt = true;
                self.history.push("EI".to_string());
            }
            0xfc => {
                let addr = self.next_memory();
                if self.s {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.history.push(format!("CM {:#06x}", addr));
            }
            0xfd => self
                .history
                .push(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xfe => {
                let value = self.read(self.pc + 1);
                let mut a = 0;
                (a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, a);
                self.pc = self.pc.wrapping_add(1);
                self.history.push(format!("CPI {:#04x}", value));
            }
            0xff => {
                self.call(0x38);
                self.history.push("RST 7".to_string());
            }
        }
        self.pc = self.pc.wrapping_add(1);
    }
}


fn disassembler(pc: usize, rom: &[u8]) -> (String, usize) {
    match rom[pc] {
        0x00 => ("NOP".to_string(), pc + 1),
        0x01 => (
            format!("LXI B, {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x02 => ("STAX B".to_string(), pc + 1),
        0x03 => ("INX B".to_string(), pc + 1),
        0x04 => ("INR B".to_string(), pc + 1),
        0x05 => ("DCR B".to_string(), pc + 1),
        0x06 => (format!("MVI B, {:#04x}", rom[pc + 1]), pc + 2),
        0x07 => ("RLC".to_string(), pc + 1),
        0x08 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x09 => ("DAD B".to_string(), pc + 1),
        0x0a => ("LDAX B".to_string(), pc + 1),
        0x0b => ("DCX B".to_string(), pc + 1),
        0x0c => ("INR C".to_string(), pc + 1),
        0x0d => ("DCR C".to_string(), pc + 1),
        0x0e => (format!("MVI C, {:#04x}", rom[pc + 1]), pc + 2),
        0x0f => ("RRC".to_string(), pc + 1),
        0x10 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x11 => (
            format!("LXI D, {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x12 => ("STAX D".to_string(), pc + 1),
        0x13 => ("INX D".to_string(), pc + 1),
        0x14 => ("INR D".to_string(), pc + 1),
        0x15 => ("DCR D".to_string(), pc + 1),
        0x16 => (format!("MVI D, {:#04x}", rom[pc + 1]), pc + 2),
        0x17 => ("RAL".to_string(), pc + 1),
        0x18 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x19 => ("DAD D".to_string(), pc + 1),
        0x1a => ("LDAX D".to_string(), pc + 1),
        0x1b => ("DCX D".to_string(), pc + 1),
        0x1c => ("INR E".to_string(), pc + 1),
        0x1d => ("DCR E".to_string(), pc + 1),
        0x1e => (format!("MVI E, {:#04x}", rom[pc + 1]), pc + 2),
        0x1f => ("RAR".to_string(), pc + 1),
        0x20 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x21 => (
            format!("LXI H, {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x22 => (
            format!("SHLD {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x23 => ("INX H".to_string(), pc + 1),
        0x24 => ("INR H".to_string(), pc + 1),
        0x25 => ("DCR H".to_string(), pc + 1),
        0x26 => (format!("MVI H, {:#04x}", rom[pc + 1]), pc + 2),
        0x27 => ("DAA".to_string(), pc + 1),
        0x28 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x29 => ("DAD H".to_string(), pc + 1),
        0x2a => (
            format!("LHLD {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x2b => ("DCX H".to_string(), pc + 1),
        0x2c => ("INR L".to_string(), pc + 1),
        0x2d => ("DCR L".to_string(), pc + 1),
        0x2e => (format!("MVI L, {:#04x}", rom[pc + 1]), pc + 2),
        0x2f => ("CMA".to_string(), pc + 1),
        0x30 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x31 => (
            format!("LXI SP, {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x32 => (
            format!("STA {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x33 => ("Invalid".to_string(), pc + 1),
        0x34 => ("INR M".to_string(), pc + 1),
        0x35 => ("DCR M".to_string(), pc + 1),
        0x36 => (format!("MVI M, {:#04x}", rom[pc + 1]), pc + 2),
        0x37 => ("STC".to_string(), pc + 1),
        0x38 => (format!("Invalid: {:#04x}", pc), pc + 1),
        0x39 => ("DAD SP".to_string(), pc + 1),
        0x3a => (
            format!("LDA {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0x3b => ("Invalid".to_string(), pc + 1),
        0x3c => ("Invalid".to_string(), pc + 1),
        0x3d => ("DCR A".to_string(), pc + 1),
        0x3e => (format!("MVI A, {:#04x}", rom[pc + 1]), pc + 2),
        0x3f => ("CMC".to_string(), pc + 1),
        0x40 => ("MOV B, B".to_string(), pc + 1),
        0x41 => ("MOV B, C".to_string(), pc + 1),
        0x42 => ("MOV B, D".to_string(), pc + 1),
        0x43 => ("MOV B, E".to_string(), pc + 1),
        0x44 => ("MOV B, H".to_string(), pc + 1),
        0x45 => ("MOV B, L".to_string(), pc + 1),
        0x46 => ("MOV B, M".to_string(), pc + 1),
        0x47 => ("MOV B, A".to_string(), pc + 1),
        0x48 => ("MOV C, B".to_string(), pc + 1),
        0x49 => ("MOV C, C".to_string(), pc + 1),
        0x4a => ("MOV C, D".to_string(), pc + 1),
        0x4b => ("MOV C, E".to_string(), pc + 1),
        0x4c => ("MOV C, H".to_string(), pc + 1),
        0x4d => ("MOV C, L".to_string(), pc + 1),
        0x4e => ("MOV C, M".to_string(), pc + 1),
        0x4f => ("MOV C, A".to_string(), pc + 1),
        0x50 => ("MOV D, B".to_string(), pc + 1),
        0x51 => ("MOV D, C".to_string(), pc + 1),
        0x52 => ("MOV D, D".to_string(), pc + 1),
        0x53 => ("MOV D, E".to_string(), pc + 1),
        0x54 => ("MOV D, H".to_string(), pc + 1),
        0x55 => ("MOV D, L".to_string(), pc + 1),
        0x56 => ("MOV D, M".to_string(), pc + 1),
        0x57 => ("MOV D, A".to_string(), pc + 1),
        0x58 => ("MOV E, B".to_string(), pc + 1),
        0x59 => ("MOV E, C".to_string(), pc + 1),
        0x5a => ("MOV E, D".to_string(), pc + 1),
        0x5b => ("MOV E, E".to_string(), pc + 1),
        0x5c => ("MOV E, H".to_string(), pc + 1),
        0x5d => ("MOV E, L".to_string(), pc + 1),
        0x5e => ("MOV E, M".to_string(), pc + 1),
        0x5f => ("MOV E, A".to_string(), pc + 1),
        0x60 => ("MOV H, B".to_string(), pc + 1),
        0x61 => ("MOV H, C".to_string(), pc + 1),
        0x62 => ("MOV H, D".to_string(), pc + 1),
        0x63 => ("MOV H, E".to_string(), pc + 1),
        0x64 => ("MOV H, H".to_string(), pc + 1),
        0x65 => ("MOV H, L".to_string(), pc + 1),
        0x66 => ("MOV H, M".to_string(), pc + 1),
        0x67 => ("MOV H, A".to_string(), pc + 1),
        0x68 => ("MOV L, B".to_string(), pc + 1),
        0x69 => ("MOV L, C".to_string(), pc + 1),
        0x6a => ("MOV L, D".to_string(), pc + 1),
        0x6b => ("MOV L, E".to_string(), pc + 1),
        0x6c => ("MOV L, H".to_string(), pc + 1),
        0x6d => ("MOV L, L".to_string(), pc + 1),
        0x6e => ("MOV L, M".to_string(), pc + 1),
        0x6f => ("MOV L, A".to_string(), pc + 1),
        0x70 => ("MOV M, B".to_string(), pc + 1),
        0x71 => ("MOV M, C".to_string(), pc + 1),
        0x72 => ("MOV M, D".to_string(), pc + 1),
        0x73 => ("MOV M, E".to_string(), pc + 1),
        0x74 => ("MOV M, H".to_string(), pc + 1),
        0x75 => ("MOV M, L".to_string(), pc + 1),
        0x76 => ("HLT".to_string(), pc + 1),
        0x77 => ("MOV M, A".to_string(), pc + 1),
        0x78 => ("MOV A, B".to_string(), pc + 1),
        0x79 => ("MOV A, C".to_string(), pc + 1),
        0x7a => ("MOV A, D".to_string(), pc + 1),
        0x7b => ("MOV A, E".to_string(), pc + 1),
        0x7c => ("MOV A, H".to_string(), pc + 1),
        0x7d => ("MOV A, L".to_string(), pc + 1),
        0x7e => ("MOV A, M".to_string(), pc + 1),
        0x7f => ("MOV A, A".to_string(), pc + 1),
        0x80 => ("ADD B".to_string(), pc + 1),
        0x81 => ("ADD C".to_string(), pc + 1),
        0x82 => ("ADD D".to_string(), pc + 1),
        0x83 => ("ADD E".to_string(), pc + 1),
        0x84 => ("ADD H".to_string(), pc + 1),
        0x85 => ("ADD L".to_string(), pc + 1),
        0x86 => ("ADD M".to_string(), pc + 1),
        0x87 => ("ADD A".to_string(), pc + 1),
        0x88 => ("ADC B".to_string(), pc + 1),
        0x89 => ("ADC C".to_string(), pc + 1),
        0x8a => ("ADC D".to_string(), pc + 1),
        0x8b => ("ADC E".to_string(), pc + 1),
        0x8c => ("ADC H".to_string(), pc + 1),
        0x8d => ("ADC L".to_string(), pc + 1),
        0x8e => ("ADC M".to_string(), pc + 1),
        0x8f => ("ADC A".to_string(), pc + 1),
        0x90 => ("SUB B".to_string(), pc + 1),
        0x91 => ("SUB C".to_string(), pc + 1),
        0x92 => ("SUB D".to_string(), pc + 1),
        0x93 => ("SUB E".to_string(), pc + 1),
        0x94 => ("SUB H".to_string(), pc + 1),
        0x95 => ("SUB L".to_string(), pc + 1),
        0x96 => ("SUB M".to_string(), pc + 1),
        0x97 => ("SUB A".to_string(), pc + 1),
        0x98 => ("SBB B".to_string(), pc + 1),
        0x99 => ("SBB C".to_string(), pc + 1),
        0x9a => ("SBB D".to_string(), pc + 1),
        0x9b => ("SBB E".to_string(), pc + 1),
        0x9c => ("SBB H".to_string(), pc + 1),
        0x9d => ("SBB L".to_string(), pc + 1),
        0x9e => ("SBB M".to_string(), pc + 1),
        0x9f => ("SBB A".to_string(), pc + 1),
        0xa0 => ("ANA B".to_string(), pc + 1),
        0xa1 => ("ANA C".to_string(), pc + 1),
        0xa2 => ("ANA D".to_string(), pc + 1),
        0xa3 => ("ANA E".to_string(), pc + 1),
        0xa4 => ("ANA H".to_string(), pc + 1),
        0xa5 => ("ANA L".to_string(), pc + 1),
        0xa6 => ("ANA M".to_string(), pc + 1),
        0xa7 => ("ANA A".to_string(), pc + 1),
        0xa8 => ("XRA B".to_string(), pc + 1),
        0xa9 => ("XRA C".to_string(), pc + 1),
        0xaa => ("XRA D".to_string(), pc + 1),
        0xab => ("XRA E".to_string(), pc + 1),
        0xac => ("XRA H".to_string(), pc + 1),
        0xad => ("XRA L".to_string(), pc + 1),
        0xae => ("XRA M".to_string(), pc + 1),
        0xaf => ("XRA A".to_string(), pc + 1),
        0xb0 => ("ORA B".to_string(), pc + 1),
        0xb1 => ("ORA C".to_string(), pc + 1),
        0xb2 => ("ORA D".to_string(), pc + 1),
        0xb3 => ("ORA E".to_string(), pc + 1),
        0xb4 => ("ORA H".to_string(), pc + 1),
        0xb5 => ("ORA L".to_string(), pc + 1),
        0xb6 => ("ORA M".to_string(), pc + 1),
        0xb7 => ("ORA A".to_string(), pc + 1),
        0xb8 => ("CMP B".to_string(), pc + 1),
        0xb9 => ("CMP C".to_string(), pc + 1),
        0xba => ("CMP D".to_string(), pc + 1),
        0xbb => ("CMP E".to_string(), pc + 1),
        0xbc => ("CMP H".to_string(), pc + 1),
        0xbd => ("CMP L".to_string(), pc + 1),
        0xbe => ("CMP M".to_string(), pc + 1),
        0xbf => ("CMP A".to_string(), pc + 1),
        0xc0 => ("RNZ".to_string(), pc + 1),
        0xc1 => ("POP B".to_string(), pc + 1),
        0xc2 => (
            format!("JNZ {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xc3 => (
            format!("JMP {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xc4 => (
            format!("CNZ {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xc5 => ("PUSH B".to_string(), pc + 1),
        0xc6 => (format!("ADI {:#04x}", rom[pc + 1]), pc + 2),
        0xc7 => ("RST 0".to_string(), pc + 1),
        0xc8 => ("RZ".to_string(), pc + 1),
        0xc9 => ("RET".to_string(), pc + 1),
        0xca => (
            format!("JZ {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xcb => (format!("Invalid: {:#04x}", rom[pc]), pc + 1),
        0xcc => (
            format!("CZ {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xcd => (
            format!("CALL {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xce => (format!("ACI {:#04x}", rom[pc + 1]), pc + 2),
        0xcf => ("RST 1".to_string(), pc + 1),
        0xd0 => ("RNC".to_string(), pc + 1),
        0xd1 => ("POP D".to_string(), pc + 1),
        0xd2 => (
            format!("JNC {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xd3 => (format!("OUT {:#04x}", rom[pc + 1]), pc + 2),
        0xd4 => (
            format!("CNC {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xd5 => ("PUSH D".to_string(), pc + 1),
        0xd6 => (format!("SUI {:#04x}", rom[pc + 1]), pc + 2),
        0xd7 => ("RST 2".to_string(), pc + 1),
        0xd8 => ("RC".to_string(), pc + 1),
        0xd9 => (format!("Invalid: {:#04x}", rom[pc]), pc + 1),
        0xda => (
            format!("JC {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xdb => (format!("IN {:#04x}", rom[pc + 1]), pc + 2),
        0xdc => (
            format!("CC {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xdd => (format!("Invalid: {:#04x}", rom[pc]), pc + 1),
        0xde => (format!("SBI {:#04x}", rom[pc + 1]), pc + 2),
        0xdf => ("RST 3".to_string(), pc + 1),
        0xe0 => ("RPO".to_string(), pc + 1),
        0xe1 => ("POP H".to_string(), pc + 1),
        0xe2 => (
            format!("JPO {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xe3 => ("XTHL".to_string(), pc + 1),
        0xe4 => (
            format!("CPO {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xe5 => ("PUSH H".to_string(), pc + 1),
        0xe6 => (format!("ANI {:#04x}", rom[pc + 1]), pc + 2),
        0xe7 => ("RST 4".to_string(), pc + 1),
        0xe8 => ("RPE".to_string(), pc + 1),
        0xe9 => ("PCHL".to_string(), pc + 1),
        0xea => (
            format!("JPE {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xeb => ("XCHG".to_string(), pc + 1),
        0xec => (
            format!("CPE {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xed => (format!("Invalid: {:#04x}", rom[pc]), pc + 1),
        0xee => (format!("XRI {:#04x}", rom[pc + 1]), pc + 2),
        0xef => ("RST 5".to_string(), pc + 1),
        0xf0 => ("RP".to_string(), pc + 1),
        0xf1 => ("POP PSW".to_string(), pc + 1),
        0xf2 => (
            format!("JP {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xf3 => ("DI".to_string(), pc + 1),
        0xf4 => (
            format!("CP {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xf5 => ("PUSH PSW".to_string(), pc + 1),
        0xf6 => (format!("ORI {:#04x}", rom[pc + 1]), pc + 2),
        0xf7 => ("RST 6".to_string(), pc + 1),
        0xf8 => ("RM".to_string(), pc + 1),
        0xf9 => ("SPHL".to_string(), pc + 1),
        0xfa => (
            format!("JM {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xfb => ("EI".to_string(), pc + 1),
        0xfc => (
            format!("CM {:#04x}{:02x}", rom[pc + 2], rom[pc + 1]),
            pc + 3,
        ),
        0xfd => (format!("Invalid: {:#04x}", rom[pc]), pc + 1),
        0xfe => (format!("CPI {:#04x}", rom[pc + 1]), pc + 2),
        0xff => ("RST 7".to_string(), pc + 1),
    }
}
