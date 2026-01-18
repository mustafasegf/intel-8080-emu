#![allow(unused)]

use std::collections::VecDeque;

use anyhow::Result;
use macroquad::prelude::*;

// Space Invaders frame buffer is 256 wide x 224 tall (in memory)
// The monitor is rotated 90 degrees CCW in the cabinet, so:
// - Frame buffer width (256) becomes screen height (256 pixels)
// - Frame buffer height (224) becomes screen width (224 pixels)
const NATIVE_WIDTH: i32 = 224;
const NATIVE_HEIGHT: i32 = 256;
const DEFAULT_PIXEL_SIZE: i32 = 3;
const GAME_WIDTH: i32 = NATIVE_WIDTH * DEFAULT_PIXEL_SIZE; // 672
const GAME_HEIGHT: i32 = NATIVE_HEIGHT * DEFAULT_PIXEL_SIZE; // 768
const DEBUG_PANEL_WIDTH: i32 = 400;

/// Calculate pixel size based on current screen dimensions
fn get_pixel_size() -> f32 {
    let sw = screen_width();
    let sh = screen_height();

    // Calculate the maximum pixel size that fits the screen
    let px_from_width = sw / NATIVE_WIDTH as f32;
    let px_from_height = sh / NATIVE_HEIGHT as f32;

    // Use the smaller to ensure it fits both dimensions
    px_from_width.min(px_from_height).max(1.0)
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Space Invaders - 8080 Emulator".to_owned(),
        fullscreen: false,
        window_resizable: true,
        window_width: GAME_WIDTH + DEBUG_PANEL_WIDTH,
        window_height: GAME_HEIGHT,
        ..Default::default()
    }
}

/// I/O Bus trait for abstracting hardware-specific I/O
trait Bus {
    fn port_in(&mut self, port: u8) -> u8;
    fn port_out(&mut self, port: u8, value: u8);
}

/// Space Invaders specific I/O hardware
///
/// I/O Ports:
/// - Read port 1: Player 1 controls, coin, start
/// - Read port 2: Player 2 controls, DIP switches
/// - Read port 3: Shift register result
/// - Write port 2: Shift amount (3 bits)
/// - Write port 3: Sound bits 1
/// - Write port 4: Shift register data
/// - Write port 5: Sound bits 2
/// - Write port 6: Watchdog
struct SpaceInvadersIO {
    /// Shift register MSB (most recently written)
    shift_msb: u8,
    /// Shift register LSB (previously written MSB)
    shift_lsb: u8,
    /// Shift amount (0-7)
    shift_offset: u8,
    /// Input port 1 state (directly mapped from keyboard)
    /// bit 0 = coin (deposit)
    /// bit 1 = P2 start
    /// bit 2 = P1 start  
    /// bit 3 = always 1 (active low, active low, active low)
    /// bit 4 = P1 fire
    /// bit 5 = P1 left
    /// bit 6 = P1 right
    /// bit 7 = not connected
    port1: u8,
    /// Input port 2 state (DIP switches + P2 controls)
    /// bit 0,1 = number of lives (DIP) - 00=3, 01=4, 10=5, 11=6
    /// bit 2 = tilt
    /// bit 3 = extra ship at (0=1500, 1=1000) (DIP)
    /// bit 4 = P2 fire
    /// bit 5 = P2 left
    /// bit 6 = P2 right
    /// bit 7 = coin info in demo (DIP)
    port2: u8,
}

impl SpaceInvadersIO {
    fn new() -> Self {
        Self {
            shift_msb: 0,
            shift_lsb: 0,
            shift_offset: 0,
            port1: 0, // no buttons pressed
            port2: 0, // DIP: 3 lives, extra ship at 1500
        }
    }

    /// Update input ports based on keyboard state
    fn update_inputs(&mut self) -> bool {
        // Reset inputs - bit 3 of port1 is always 1
        self.port1 = 0x08;
        self.port2 = 0;

        let mut any_key = false;

        // Coin - C key
        if is_key_down(KeyCode::C) {
            self.port1 |= 0x01;
            any_key = true;
        }

        // P1 Start - 1 key
        if is_key_down(KeyCode::Key1) {
            self.port1 |= 0x04;
            any_key = true;
        }

        // P2 Start - 2 key
        if is_key_down(KeyCode::Key2) {
            self.port1 |= 0x02;
            any_key = true;
        }

        // P1 Fire - Space or W
        if is_key_down(KeyCode::Space) || is_key_down(KeyCode::W) {
            self.port1 |= 0x10;
            any_key = true;
        }

        // P1 Left - Left arrow or A
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.port1 |= 0x20;
            any_key = true;
        }

        // P1 Right - Right arrow or D
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.port1 |= 0x40;
            any_key = true;
        }

        // P2 Fire - I key
        if is_key_down(KeyCode::I) {
            self.port2 |= 0x10;
            any_key = true;
        }

        // P2 Left - J key
        if is_key_down(KeyCode::J) {
            self.port2 |= 0x20;
            any_key = true;
        }

        // P2 Right - L key
        if is_key_down(KeyCode::L) {
            self.port2 |= 0x40;
            any_key = true;
        }

        // Tilt - T key
        if is_key_down(KeyCode::T) {
            self.port2 |= 0x04;
            any_key = true;
        }

        any_key
    }
}

impl Bus for SpaceInvadersIO {
    fn port_in(&mut self, port: u8) -> u8 {
        match port {
            0 => {
                // INP0 - Not used by game, but mapped
                // bit 0 = DIP4 (self-test at power up)
                // bits 1-3 = always 1
                // bits 4-6 = P1 controls (duplicate?)
                0b0000_1110
            }
            1 => {
                // INP1 - Player 1 controls
                self.port1
            }
            2 => {
                // INP2 - Player 2 controls + DIP switches
                self.port2
            }
            3 => {
                // Shift register read
                // Reference: ((this._register << this._bitShift) >> 8) & 0xFF
                let shift = ((self.shift_msb as u16) << 8) | (self.shift_lsb as u16);
                ((shift << self.shift_offset) >> 8) as u8
            }
            _ => {
                // Unknown port
                0
            }
        }
    }

    fn port_out(&mut self, port: u8, value: u8) {
        match port {
            2 => {
                // Shift amount (only lower 3 bits used)
                self.shift_offset = value & 0x07;
            }
            3 => {
                // Sound port 1
                // bit 0 = UFO repeating sound
                // bit 1 = player shot
                // bit 2 = player explosion
                // bit 3 = invader explosion
                // bit 4 = extended play
                // bit 5 = amp enable
                // TODO: implement sound
            }
            4 => {
                // Shift data
                // Writing to port 4 shifts MSB into LSB, and the new value into MSB
                self.shift_lsb = self.shift_msb;
                self.shift_msb = value;
            }
            5 => {
                // Sound port 2
                // bit 0-3 = fleet movement sounds
                // bit 4 = UFO hit
                // TODO: implement sound
            }
            6 => {
                // Watchdog - any write resets the watchdog timer
                // Not critical for emulation
            }
            _ => {
                // Unknown port - ignore
            }
        }
    }
}

/// Embed ROM at compile time for web builds
/// For native builds, we try to load from file first, then fall back to embedded
#[cfg(target_arch = "wasm32")]
const EMBEDDED_ROM: Option<&[u8]> = Some(include_bytes!("../rom/space-invaders/invaders"));

#[cfg(not(target_arch = "wasm32"))]
const EMBEDDED_ROM: Option<&[u8]> = None;

fn load_rom() -> Vec<u8> {
    // Try embedded ROM first (required for WASM)
    if let Some(rom) = EMBEDDED_ROM {
        return rom.to_vec();
    }

    // Try loading from file (native only)
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(rom) = std::fs::read("./rom/space-invaders/invaders") {
            return rom;
        }
    }

    panic!("Unable to load ROM! Place 'invaders' ROM file in ./rom/space-invaders/");
}

#[macroquad::main(window_conf)]
async fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("Space Invaders - 8080 Emulator");
        println!("Controls:");
        println!("  C = Insert Coin");
        println!("  1 = 1 Player Start");
        println!("  2 = 2 Player Start");
        println!("  A/Left  = Move Left");
        println!("  D/Right = Move Right");
        println!("  Space/W = Fire");
    }

    let rom = load_rom();

    let mut cpu = Cpu8080::new();
    let mut io = SpaceInvadersIO::new();

    cpu.load(&rom);

    // Space Invaders runs at ~2MHz, 60Hz display
    // That's ~33,333 cycles per frame, split between two interrupts
    // RST 8 (0xCF) at mid-screen, RST 10 (0xD7) at vblank
    const CYCLES_PER_HALF_FRAME: u64 = 16_667; // ~2MHz / 60Hz / 2

    let mut next_interrupt: u8 = 0xcf; // Alternate between 0xCF and 0xD7
    let mut cycle_count: u64 = 0;

    // Emulator control state
    let mut paused = false;
    let mut step_once = false;
    let mut fire_vblank = false;
    let mut fire_half_vblank = false;

    loop {
        // Update input state from keyboard
        let any_key = io.update_inputs();

        // Keyboard shortcuts for emulator control
        if is_key_pressed(KeyCode::P) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::N) && paused {
            step_once = true;
        }
        if is_key_pressed(KeyCode::R) {
            // Reset
            cpu = Cpu8080::new();
            cpu.load(&rom);
            io = SpaceInvadersIO::new();
            next_interrupt = 0xcf;
            cycle_count = 0;
            paused = false;
        }

        // Only run emulation if not paused (or stepping)
        if !paused || step_once {
            if step_once {
                // Single step: execute one instruction
                if !cpu.halt {
                    cpu.step(&mut io);
                }
                step_once = false;
            } else {
                // Run CPU until we've executed enough cycles for half a frame
                while cycle_count < CYCLES_PER_HALF_FRAME {
                    if !cpu.halt {
                        cpu.step(&mut io);
                        cycle_count += 4; // Approximate: average ~4 cycles per instruction
                    } else {
                        // If halted, just count cycles
                        cycle_count += 4;
                    }
                }
                cycle_count -= CYCLES_PER_HALF_FRAME;

                // Fire interrupt - it will be queued and processed when interrupts are enabled
                cpu.generate_interrupt(next_interrupt);
                // Alternate between RST 8 (0xCF) and RST 10 (0xD7)
                next_interrupt = if next_interrupt == 0xcf { 0xd7 } else { 0xcf };
            }
        }

        // Handle manual interrupt buttons (even when paused)
        if fire_vblank {
            cpu.generate_interrupt(0xd7); // RST 10 - vblank
            fire_vblank = false;
        }
        if fire_half_vblank {
            cpu.generate_interrupt(0xcf); // RST 8 - half vblank
            fire_half_vblank = false;
        }

        // Render every frame (not just on vblank) so UI is responsive
        {
            // Render the screen
            clear_background(BLACK);

            // Calculate dynamic pixel size based on screen dimensions
            let pixel_size = get_pixel_size();
            let game_width = NATIVE_WIDTH as f32 * pixel_size;
            let game_height = NATIVE_HEIGHT as f32 * pixel_size;

            // VRAM is at 0x2400-0x3FFF (7K bytes = 256x224 pixels, 1 bit per pixel)
            // Memory layout: 32 bytes per row (32*8=256 pixels width), 224 rows (height)
            // The monitor in the cabinet is rotated 90 degrees counter-clockwise
            //
            // In memory (before rotation):
            //   - 224 rows of 32 bytes each (256 bits = 256 pixels wide)
            //   - Each byte is a vertical strip of 8 pixels
            //   - Bit 0 is at the top of that strip, bit 7 at the bottom
            //
            // For CCW rotation: original (x, y) -> screen (y, width-1-x)
            // But we also need to flip because the frame buffer origin differs

            for byte_idx in 0..0x1c00 {
                let byte = cpu.memory[0x2400 + byte_idx];
                if byte == 0 {
                    continue; // Skip empty bytes for performance
                }

                for bit in 0..8 {
                    if byte & (1 << bit) != 0 {
                        // Calculate original position in the 256x224 frame buffer
                        // byte_idx / 32 = row (0..223), byte_idx % 32 = column of bytes (0..31)
                        let row = byte_idx / 32; // 0..223 (this is the X in rotated view)
                        let col = byte_idx % 32; // 0..31
                        let original_x = col * 8 + bit; // 0..255 (pixel X before rotation)
                        let original_y = row; // 0..223 (pixel Y before rotation)

                        // Rotate 90 degrees counter-clockwise for the cabinet display
                        // CCW rotation: (x, y) -> (y, maxX - x)
                        // maxX = 255, so new position is (original_y, 255 - original_x)
                        let screen_x = original_y; // 0..223
                        let screen_y = 255 - original_x; // 0..255

                        // Apply pixel scaling
                        let x = screen_x as f32 * pixel_size;
                        let y = screen_y as f32 * pixel_size;

                        draw_rectangle(x, y, pixel_size, pixel_size, WHITE);
                    }
                }
            }

            // Only draw debug panel if screen is wide enough (desktop mode)
            let show_debug_panel = screen_width() > game_width + 100.0;

            if show_debug_panel {
                // Draw debug panel on the right side
                let panel_x = game_width;
                let panel_bg = Color::new(0.1, 0.1, 0.15, 1.0);
                draw_rectangle(
                    panel_x,
                    0.0,
                    DEBUG_PANEL_WIDTH as f32,
                    game_height,
                    panel_bg,
                );

                // Draw separator line
                draw_line(panel_x, 0.0, panel_x, game_height, 2.0, GREEN);

                let font_size = 16.0;
                let line_height = 20.0;
                let mut y = 25.0;
                let x = panel_x + 10.0;
                let label_color = Color::new(0.6, 0.6, 0.6, 1.0);
                let value_color = GREEN;
                let header_color = Color::new(0.0, 1.0, 0.5, 1.0);

                // Title
                draw_text("CPU STATE", x, y, 20.0, header_color);
                y += line_height + 10.0;

                // Program Counter & Stack Pointer
                draw_text("PC:", x, y, font_size, label_color);
                draw_text(
                    &format!("{:04X}", cpu.pc),
                    x + 40.0,
                    y,
                    font_size,
                    value_color,
                );
                draw_text("SP:", x + 100.0, y, font_size, label_color);
                draw_text(
                    &format!("{:04X}", cpu.sp),
                    x + 140.0,
                    y,
                    font_size,
                    value_color,
                );
                y += line_height;

                // Interrupts
                draw_text("INT:", x, y, font_size, label_color);
                let int_status = if cpu.interrupt { "ENABLED" } else { "DISABLED" };
                let int_color = if cpu.interrupt { GREEN } else { RED };
                draw_text(int_status, x + 50.0, y, font_size, int_color);
                y += line_height + 10.0;

                // Registers header
                draw_text("REGISTERS", x, y, 18.0, header_color);
                y += line_height + 5.0;

                // Registers in a grid
                let regs = [
                    ("A", cpu.a),
                    ("B", cpu.b),
                    ("C", cpu.c),
                    ("D", cpu.d),
                    ("E", cpu.e),
                    ("H", cpu.h),
                    ("L", cpu.l),
                ];
                for (i, (name, val)) in regs.iter().enumerate() {
                    let col = (i % 4) as f32;
                    let row = (i / 4) as f32;
                    let rx = x + col * 95.0;
                    let ry = y + row * line_height;
                    draw_text(&format!("{}:", name), rx, ry, font_size, label_color);
                    draw_text(
                        &format!("{:02X}", val),
                        rx + 25.0,
                        ry,
                        font_size,
                        value_color,
                    );
                }
                y += (line_height * 2.0) + 10.0;

                // Register pairs
                draw_text("BC:", x, y, font_size, label_color);
                draw_text(
                    &format!("{:04X}", cpu.bc()),
                    x + 35.0,
                    y,
                    font_size,
                    value_color,
                );
                draw_text("DE:", x + 100.0, y, font_size, label_color);
                draw_text(
                    &format!("{:04X}", cpu.de()),
                    x + 135.0,
                    y,
                    font_size,
                    value_color,
                );
                draw_text("HL:", x + 200.0, y, font_size, label_color);
                draw_text(
                    &format!("{:04X}", cpu.hl()),
                    x + 235.0,
                    y,
                    font_size,
                    value_color,
                );
                y += line_height + 10.0;

                // Flags header
                draw_text("FLAGS", x, y, 18.0, header_color);
                y += line_height + 5.0;

                // Flags
                let flags = [
                    ("Z", cpu.z),
                    ("S", cpu.s),
                    ("P", cpu.p),
                    ("CY", cpu.cy),
                    ("AC", cpu.ac),
                ];
                for (i, (name, val)) in flags.iter().enumerate() {
                    let fx = x + (i as f32) * 70.0;
                    draw_text(&format!("{}:", name), fx, y, font_size, label_color);
                    let flag_val = if *val { "1" } else { "0" };
                    let flag_color = if *val { GREEN } else { RED };
                    draw_text(flag_val, fx + 30.0, y, font_size, flag_color);
                }
                y += line_height + 15.0;

                // I/O Ports
                draw_text("I/O PORTS", x, y, 18.0, header_color);
                y += line_height + 5.0;
                draw_text("P1:", x, y, font_size, label_color);
                draw_text(
                    &format!("{:02X} ({:08b})", io.port1, io.port1),
                    x + 35.0,
                    y,
                    font_size,
                    value_color,
                );
                y += line_height;
                draw_text("P2:", x, y, font_size, label_color);
                draw_text(
                    &format!("{:02X} ({:08b})", io.port2, io.port2),
                    x + 35.0,
                    y,
                    font_size,
                    value_color,
                );
                y += line_height;
                draw_text("Shift:", x, y, font_size, label_color);
                let shift_val =
                    ((io.shift_msb as u16) << 8 | io.shift_lsb as u16) >> (8 - io.shift_offset);
                draw_text(
                    &format!("{:02X} (off:{})", shift_val as u8, io.shift_offset),
                    x + 60.0,
                    y,
                    font_size,
                    value_color,
                );
                y += line_height + 15.0;

                // Control buttons
                draw_text("CONTROLS", x, y, 18.0, header_color);
                y += line_height + 5.0;

                // Status indicator
                let status_text = if paused { "PAUSED" } else { "RUNNING" };
                let status_color = if paused { YELLOW } else { GREEN };
                draw_text("Status:", x, y, font_size, label_color);
                draw_text(status_text, x + 70.0, y, font_size, status_color);
                y += line_height + 5.0;

                // Button helper
                let mouse_pos = mouse_position();
                let mouse_clicked = is_mouse_button_pressed(MouseButton::Left);

                let mut draw_button =
                    |bx: f32, by: f32, bw: f32, bh: f32, label: &str, enabled: bool| -> bool {
                        let hover = mouse_pos.0 >= bx
                            && mouse_pos.0 <= bx + bw
                            && mouse_pos.1 >= by
                            && mouse_pos.1 <= by + bh;
                        let bg_color = if !enabled {
                            Color::new(0.2, 0.2, 0.2, 1.0)
                        } else if hover {
                            Color::new(0.3, 0.5, 0.3, 1.0)
                        } else {
                            Color::new(0.2, 0.3, 0.2, 1.0)
                        };
                        let border_color = if enabled {
                            GREEN
                        } else {
                            Color::new(0.3, 0.3, 0.3, 1.0)
                        };
                        let text_color = if enabled {
                            WHITE
                        } else {
                            Color::new(0.4, 0.4, 0.4, 1.0)
                        };

                        draw_rectangle(bx, by, bw, bh, bg_color);
                        draw_rectangle_lines(bx, by, bw, bh, 2.0, border_color);
                        let text_x = bx + (bw - label.len() as f32 * 7.0) / 2.0;
                        let text_y = by + bh / 2.0 + 5.0;
                        draw_text(label, text_x, text_y, 16.0, text_color);

                        enabled && hover && mouse_clicked
                    };

                let btn_w = 120.0;
                let btn_h = 28.0;
                let btn_spacing = 5.0;

                // Row 1: Play/Pause and Reset
                let btn_y = y;
                if paused {
                    if draw_button(x, btn_y, btn_w, btn_h, "PLAY", true) {
                        paused = false;
                    }
                } else {
                    if draw_button(x, btn_y, btn_w, btn_h, "PAUSE", true) {
                        paused = true;
                    }
                }
                if draw_button(x + btn_w + btn_spacing, btn_y, btn_w, btn_h, "RESET", true) {
                    cpu = Cpu8080::new();
                    cpu.load(&rom);
                    io = SpaceInvadersIO::new();
                    next_interrupt = 0xcf;
                    cycle_count = 0;
                    paused = false;
                }
                y += btn_h + btn_spacing;

                // Row 2: Step (only when paused)
                if draw_button(x, y, btn_w * 2.0 + btn_spacing, btn_h, "STEP (N)", paused) {
                    step_once = true;
                }
                y += btn_h + btn_spacing;

                // Row 3: Interrupt buttons
                if draw_button(x, y, btn_w, btn_h, "VBLANK INT", paused) {
                    fire_vblank = true;
                }
                if draw_button(
                    x + btn_w + btn_spacing,
                    y,
                    btn_w,
                    btn_h,
                    "HALF-VBL INT",
                    paused,
                ) {
                    fire_half_vblank = true;
                }
                y += btn_h + btn_spacing;
                y += 10.0;

                // Keyboard shortcuts help
                draw_text("Keys: P=Pause  N=Step  R=Reset", x, y, 12.0, label_color);
                y += line_height + 10.0;

                // Disassembly header
                draw_text("DISASSEMBLY (Last 15)", x, y, 18.0, header_color);
                y += line_height + 5.0;

                // Show last 15 instructions
                let history_len = cpu.history.len();
                let start = if history_len > 15 {
                    history_len - 15
                } else {
                    0
                };
                let trace_color = Color::new(1.0, 0.8, 0.4, 1.0);
                for instr in cpu.history.iter().skip(start) {
                    draw_text(instr, x, y, 14.0, trace_color);
                    y += 16.0;
                    if y > GAME_HEIGHT as f32 - 20.0 {
                        break;
                    }
                }
            } // end if show_debug_panel

            next_frame().await;
        }
    }
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

    /// Interrupts enabled (controlled by EI/DI)
    pub interrupt: bool,
    /// Interrupt is pending/waiting to be processed
    pub interrupt_pending: bool,
    /// Opcode of pending interrupt (e.g., 0xCF for RST 1)
    pub pending_interrupt_opcode: u8,

    pub halt: bool,

    pub memory: [u8; 0x10000],
    /// special for space invaders
    pub mirror: u16,

    pub history: VecDeque<String>,
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
    /// Maximum number of history entries to keep (to prevent OOM)
    const MAX_HISTORY_SIZE: usize = 100;

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
            interrupt_pending: false,
            pending_interrupt_opcode: 0,
            halt: false,
            memory: [0; 0x10000],
            mirror: 0,
            history: VecDeque::with_capacity(Self::MAX_HISTORY_SIZE + 1),
        }
    }

    /// Push an instruction to history, keeping only the last MAX_HISTORY_SIZE entries
    fn push_history(&mut self, instr: String) {
        if self.history.len() >= Self::MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(instr);
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
        let addr = addr as usize;
        // Handle RAM mirror: 0x4000-0x5FFF mirrors 0x2000-0x3FFF
        if addr >= 0x4000 && addr < 0x6000 {
            return self.memory[addr - 0x2000];
        }
        if addr >= 0x6000 {
            return 0; // Nothing above 0x6000
        }
        self.memory[addr]
    }

    fn write(&mut self, addr: u16, value: u8) {
        let addr = addr as usize;
        // ROM is 0x0000-0x1FFF (write protected)
        // RAM is 0x2000-0x3FFF (writable)
        // RAM mirror is 0x4000-0x5FFF -> maps to 0x2000-0x3FFF
        if addr >= 0x2000 && addr < 0x4000 {
            self.memory[addr] = value;
        } else if addr >= 0x4000 && addr < 0x6000 {
            self.memory[addr - 0x2000] = value;
        }
        // Writes to ROM (0x0000-0x1FFF) are ignored
        // Writes above 0x6000 are ignored
    }

    fn next_memory(&self) -> u16 {
        self.read(self.pc + 1) as u16 | (self.read(self.pc + 2) as u16) << 8
    }

    fn pop(&mut self) -> u16 {
        // 8080 little-endian: low byte at SP, high byte at SP+1
        let low = self.read(self.sp) as u16;
        let high = self.read(self.sp + 1) as u16;
        self.sp = self.sp.wrapping_add(2);
        (high << 8) | low
    }

    fn push(&mut self, value: u16) {
        // 8080 little-endian: push high byte first (to SP-1), then low byte (to SP-2)
        // Result: low byte at SP, high byte at SP+1
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (value >> 8) as u8); // high byte
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, value as u8); // low byte
    }

    fn call(&mut self, addr: u16) {
        // Return address is PC + 3 (after the 3-byte CALL instruction)
        let ret_addr = self.pc.wrapping_add(3);
        // Push return address (high byte first, then low byte)
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (ret_addr >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, ret_addr as u8);
        self.pc = addr.wrapping_sub(1);
    }

    fn rst(&mut self, vector: u16) {
        // Return address is PC + 1 (after the 1-byte RST instruction)
        let ret_addr = self.pc.wrapping_add(1);
        // Push return address (high byte first, then low byte)
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (ret_addr >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, ret_addr as u8);
        self.pc = vector.wrapping_sub(1);
    }

    /// Generate an interrupt with the given opcode (typically RST instruction)
    /// The opcode is queued and will be processed at the start of the next step()
    /// when interrupts are enabled. For RST 1 pass 0xCF, for RST 2 pass 0xD7.
    fn generate_interrupt(&mut self, opcode: u8) {
        // Queue the interrupt - it will be processed when interrupts are enabled
        self.interrupt_pending = true;
        self.pending_interrupt_opcode = opcode;
    }

    /// Process a pending interrupt if interrupts are enabled
    fn process_interrupt(&mut self) {
        if !self.interrupt_pending || !self.interrupt {
            return;
        }
        // Clear pending flag and disable interrupts
        self.interrupt_pending = false;
        self.interrupt = false;
        // Push PC onto stack (high byte first, then low byte)
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (self.pc >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, self.pc as u8);
        // Jump to interrupt vector
        // RST n jumps to 8*n: RST 0=0x00, RST 1=0x08, RST 2=0x10, etc.
        // Opcode 0xC7 = RST 0, 0xCF = RST 1, 0xD7 = RST 2, etc.
        let vector = (self.pending_interrupt_opcode & 0x38) as u16;
        self.pc = vector;
        self.halt = false;
    }

    fn step(&mut self, io: &mut dyn Bus) {
        // Process any pending interrupt at the start of each instruction
        self.process_interrupt();

        match self.read(self.pc) {
            0x00 => self.push_history("NOP".to_string()),
            0x01 => {
                let addr = self.next_memory();
                self.set_bc(addr);
                self.pc = self.pc.wrapping_add(2);
                self.push_history(format!("LXI B, {:#06x}", addr));
            }
            0x02 => {
                self.write(self.bc(), self.a);
                self.push_history("STAX B".to_string());
            }
            0x03 => {
                self.set_bc(self.bc().wrapping_add(1));
                self.push_history("INX B".to_string());
            }
            0x04 => {
                self.b = self.b.wrapping_add(1);
                flag!(self, self.b);
                self.push_history("INR B".to_string());
            }
            0x05 => {
                self.b = self.b.wrapping_sub(1);
                flag!(self, self.b);
                self.push_history("DCR B".to_string());
            }
            0x06 => {
                self.b = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI B, {:#04x}", self.b));
            }
            0x07 => {
                self.cy = self.a & (1 << 7) != 0;
                self.a = self.a.rotate_left(1);
                self.push_history("RLC".to_string());
            }
            0x08 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x09 => {
                let (hl, overflow) = self.hl().overflowing_add(self.bc());
                self.set_hl(hl);
                self.cy = overflow;
                self.push_history("DAD B".to_string());
            }
            0x0a => {
                self.a = self.read(self.bc());
                self.push_history("LDAX B".to_string());
            }
            0x0b => {
                self.set_bc(self.bc().wrapping_sub(1));
                self.push_history("DCX B".to_string());
            }
            0x0c => {
                self.c = self.c.wrapping_add(1);
                flag!(self, self.c);
                self.push_history("INR C".to_string());
            }
            0x0d => {
                self.c = self.c.wrapping_sub(1);
                flag!(self, self.c);
                self.push_history("DCR C".to_string());
            }
            0x0e => {
                self.c = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI C, {:#04x}", self.c));
            }
            0x0f => {
                // RRC: bit 0 goes to carry and also wraps to bit 7
                self.cy = self.a & 1 != 0;
                self.a = self.a.rotate_right(1);
                self.push_history("RRC".to_string());
            }
            0x10 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x11 => {
                let addr = self.next_memory();
                self.set_de(addr);
                self.pc = self.pc.wrapping_add(2);
                self.push_history(format!("LXI D, {:#06x}", addr));
            }
            0x12 => {
                self.write(self.de(), self.a);
                self.push_history("STAX D".to_string());
            }
            0x13 => {
                self.set_de(self.de().wrapping_add(1));
                self.push_history("INX D".to_string());
            }
            0x14 => {
                self.d = self.d.wrapping_add(1);
                flag!(self, self.d);
                self.push_history("INR D".to_string());
            }
            0x15 => {
                self.d = self.d.wrapping_sub(1);
                flag!(self, self.d);
                self.push_history("DCR D".to_string());
            }
            0x16 => {
                self.d = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI D, {:#04x}", self.d));
            }
            0x17 => {
                // RAL: Rotate left through carry
                // MSB goes to carry, old carry goes to LSB
                let old_cy = self.cy as u8;
                self.cy = self.a & 0x80 != 0;
                self.a = (self.a << 1) | old_cy;
                self.push_history("RAL".to_string());
            }
            0x18 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x19 => {
                let (hl, overflow) = self.hl().overflowing_add(self.de());
                self.set_hl(hl);
                self.cy = overflow;
                self.push_history("DAD D".to_string());
            }
            0x1a => {
                self.a = self.read(self.de());
                self.push_history("LDAX D".to_string());
            }
            0x1b => {
                self.set_de(self.de().wrapping_sub(1));
                self.push_history("DCX D".to_string());
            }
            0x1c => {
                self.e = self.e.wrapping_add(1);
                flag!(self, self.e);
                self.push_history("INR E".to_string());
            }
            0x1d => {
                self.e = self.e.wrapping_sub(1);
                flag!(self, self.e);
                self.push_history("DCR E".to_string());
            }
            0x1e => {
                self.e = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI E, {:#04x}", self.e));
            }
            0x1f => {
                // RAR: Rotate right through carry
                // LSB goes to carry, old carry goes to MSB
                let old_cy = self.cy as u8;
                self.cy = self.a & 1 != 0;
                self.a = (self.a >> 1) | (old_cy << 7);
                self.push_history("RAR".to_string());
            }
            0x20 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x21 => {
                let addr = self.next_memory();
                self.set_hl(addr);
                self.pc = self.pc.wrapping_add(2);
                self.push_history(format!("LXI H, {:#06x}", addr));
            }
            0x22 => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
                self.write(addr, self.l);
                self.write(addr.wrapping_add(1), self.h);
                self.push_history(format!("SHLD {:#06x}", addr));
            }
            0x23 => {
                self.set_hl(self.hl().wrapping_add(1));
                self.push_history("INX H".to_string());
            }
            0x24 => {
                self.h = self.h.wrapping_add(1);
                flag!(self, self.h);
                self.push_history("INR H".to_string());
            }
            0x25 => {
                self.h = self.h.wrapping_sub(1);
                flag!(self, self.h);
                self.push_history("DCR H".to_string());
            }
            0x26 => {
                self.h = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI H, {:#04x}", self.h));
            }
            0x27 => {
                // DAA: Decimal Adjust Accumulator
                // Step 1: If low nibble > 9 or AC flag is set, add 6
                if (self.a & 0x0f) > 9 || self.ac {
                    let low_result = self.a.wrapping_add(0x06);
                    self.ac = (self.a & 0x0f) + 0x06 > 0x0f;
                    self.a = low_result;
                }
                // Step 2: If high nibble > 9 or CY flag is set, add 0x60
                if (self.a >> 4) > 9 || self.cy {
                    let (high_result, overflow) = self.a.overflowing_add(0x60);
                    if overflow {
                        self.cy = true;
                    }
                    self.a = high_result;
                }
                // Set other flags based on result
                self.z = self.a == 0;
                self.s = self.a & 0x80 != 0;
                self.p = self.a.count_ones() % 2 == 0;
                self.push_history("DAA".to_string());
            }
            0x28 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x29 => {
                let (hl, overflow) = self.hl().overflowing_add(self.hl());
                self.set_hl(hl);
                self.cy = overflow;
                self.push_history("DAD H".to_string());
            }
            0x2a => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
                self.l = self.read(addr);
                self.h = self.read(addr.wrapping_add(1));
                self.push_history(format!("LHLD {:#06x}", addr));
            }
            0x2b => {
                self.set_hl(self.hl().wrapping_sub(1));
                self.push_history("DCX H".to_string());
            }
            0x2c => {
                self.l = self.l.wrapping_add(1);
                flag!(self, self.l);
                self.push_history("INR L".to_string());
            }
            0x2d => {
                self.l = self.l.wrapping_sub(1);
                flag!(self, self.l);
                self.push_history("DCR L".to_string());
            }
            0x2e => {
                self.l = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI L, {:#04x}", self.l));
            }
            0x2f => {
                self.a = !self.a;
                self.push_history("CMA".to_string());
            }
            0x30 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x31 => {
                self.sp = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
                self.push_history(format!("LXI SP, {:#06x}", self.sp));
            }
            0x32 => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
                self.write(addr, self.a);
                self.push_history(format!("STA {:#06x}", addr));
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                self.push_history("INX SP".to_string());
            }
            0x34 => {
                let addr = self.hl();
                let value = self.read(addr).wrapping_add(1);
                self.write(addr, value);
                self.z = value == 0;
                self.s = value & (1 << 7) != 0;
                self.p = value.count_ones() % 2 == 0;
                self.ac = value & 0x0f == 0; // AC set if low nibble wrapped from 0xF to 0x0
                self.push_history("INR M".to_string());
            }
            0x35 => {
                let addr = self.hl();
                let value = self.read(addr).wrapping_sub(1);
                self.write(addr, value);
                self.z = value == 0;
                self.s = value & (1 << 7) != 0;
                self.p = value.count_ones() % 2 == 0;
                self.ac = value & 0x0f != 0x0f; // AC set if no borrow from bit 4
                self.push_history("DCR M".to_string());
            }
            0x36 => {
                let addr = self.hl();
                let value = self.read(self.pc + 1);
                self.write(addr, value);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI M, {:#04x}", value));
            }
            0x37 => {
                self.cy = true;
                self.push_history("STC".to_string());
            }
            0x38 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0x39 => {
                let (hl, overflow) = self.hl().overflowing_add(self.sp);
                self.set_hl(hl);
                self.cy = overflow;
                self.push_history("DAD SP".to_string());
            }
            0x3a => {
                let addr = self.next_memory();
                self.pc = self.pc.wrapping_add(2);
                self.a = self.read(addr);
                self.push_history(format!("LDA {:#06x}", addr));
            }
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                self.push_history("DCX SP".to_string());
            }
            0x3c => {
                self.a = self.a.wrapping_add(1);
                flag!(self, self.a);
                self.push_history("INR A".to_string());
            }
            0x3d => {
                self.a = self.a.wrapping_sub(1);
                flag!(self, self.a);
                self.push_history("DCR A".to_string());
            }
            0x3e => {
                self.a = self.read(self.pc + 1);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("MVI A, {:#04x}", self.a));
            }
            0x3f => {
                // CMC: Complement Carry flag
                self.cy = !self.cy;
                self.push_history("CMC".to_string());
            }
            0x40 => {
                self.b = self.b;
                self.push_history("MOV B, B".to_string());
            }
            0x41 => {
                self.b = self.c;
                self.push_history("MOV B, C".to_string());
            }
            0x42 => {
                self.b = self.d;
                self.push_history("MOV B, D".to_string());
            }
            0x43 => {
                self.b = self.e;
                self.push_history("MOV B, E".to_string());
            }
            0x44 => {
                self.b = self.h;
                self.push_history("MOV B, H".to_string());
            }
            0x45 => {
                self.b = self.l;
                self.push_history("MOV B, L".to_string());
            }
            0x46 => {
                self.b = self.read(self.hl());
                self.push_history("MOV B, M".to_string());
            }
            0x47 => {
                self.b = self.a;
                self.push_history("MOV B, A".to_string());
            }
            0x48 => {
                self.c = self.b;
                self.push_history("MOV C, B".to_string());
            }
            0x49 => {
                self.c = self.c;
                self.push_history("MOV C, C".to_string());
            }
            0x4a => {
                self.c = self.d;
                self.push_history("MOV C, D".to_string());
            }
            0x4b => {
                self.c = self.e;
                self.push_history("MOV C, E".to_string());
            }
            0x4c => {
                self.c = self.h;
                self.push_history("MOV C, H".to_string());
            }
            0x4d => {
                self.c = self.l;
                self.push_history("MOV C, L".to_string());
            }
            0x4e => {
                self.c = self.read(self.hl());
                self.push_history("MOV C, M".to_string());
            }
            0x4f => {
                self.c = self.a;
                self.push_history("MOV C, A".to_string());
            }
            0x50 => {
                self.d = self.b;
                self.push_history("MOV D, B".to_string());
            }
            0x51 => {
                self.d = self.c;
                self.push_history("MOV D, C".to_string());
            }
            0x52 => {
                self.d = self.d;
                self.push_history("MOV D, D".to_string());
            }
            0x53 => {
                self.d = self.e;
                self.push_history("MOV D, E".to_string());
            }
            0x54 => {
                self.d = self.h;
                self.push_history("MOV D, H".to_string());
            }
            0x55 => {
                self.d = self.l;
                self.push_history("MOV D, L".to_string());
            }
            0x56 => {
                self.d = self.read(self.hl());
                self.push_history("MOV D, M".to_string());
            }
            0x57 => {
                self.d = self.a;
                self.push_history("MOV D, A".to_string());
            }
            0x58 => {
                self.e = self.b;
                self.push_history("MOV E, B".to_string());
            }
            0x59 => {
                self.e = self.c;
                self.push_history("MOV E, C".to_string());
            }
            0x5a => {
                self.e = self.d;
                self.push_history("MOV E, D".to_string());
            }
            0x5b => {
                self.e = self.e;
                self.push_history("MOV E, E".to_string());
            }
            0x5c => {
                self.e = self.h;
                self.push_history("MOV E, H".to_string());
            }
            0x5d => {
                self.e = self.l;
                self.push_history("MOV E, L".to_string());
            }
            0x5e => {
                self.e = self.read(self.hl());
                self.push_history("MOV E, M".to_string());
            }
            0x5f => {
                self.e = self.a;
                self.push_history("MOV E, A".to_string());
            }
            0x60 => {
                self.h = self.b;
                self.push_history("MOV H, B".to_string());
            }
            0x61 => {
                self.h = self.c;
                self.push_history("MOV H, C".to_string());
            }
            0x62 => {
                self.h = self.d;
                self.push_history("MOV H, D".to_string());
            }
            0x63 => {
                self.h = self.e;
                self.push_history("MOV H, E".to_string());
            }
            0x64 => {
                self.h = self.h;
                self.push_history("MOV H, H".to_string());
            }
            0x65 => {
                self.h = self.l;
                self.push_history("MOV H, L".to_string());
            }
            0x66 => {
                self.h = self.read(self.hl());
                self.push_history("MOV H, M".to_string());
            }
            0x67 => {
                self.h = self.a;
                self.push_history("MOV H, A".to_string());
            }
            0x68 => {
                self.l = self.b;
                self.push_history("MOV L, B".to_string());
            }
            0x69 => {
                self.l = self.c;
                self.push_history("MOV L, C".to_string());
            }
            0x6a => {
                self.l = self.d;
                self.push_history("MOV L, D".to_string());
            }
            0x6b => {
                self.l = self.e;
                self.push_history("MOV L, E".to_string());
            }
            0x6c => {
                self.l = self.h;
                self.push_history("MOV L, H".to_string());
            }
            0x6d => {
                self.l = self.l;
                self.push_history("MOV L, L".to_string());
            }
            0x6e => {
                self.l = self.read(self.hl());
                self.push_history("MOV L, M".to_string());
            }
            0x6f => {
                self.l = self.a;
                self.push_history("MOV L, A".to_string());
            }
            0x70 => {
                self.write(self.hl(), self.b);
                self.push_history("MOV M, B".to_string());
            }
            0x71 => {
                self.write(self.hl(), self.c);
                self.push_history("MOV M, C".to_string());
            }
            0x72 => {
                self.write(self.hl(), self.d);
                self.push_history("MOV M, D".to_string());
            }
            0x73 => {
                self.write(self.hl(), self.e);
                self.push_history("MOV M, E".to_string());
            }
            0x74 => {
                self.write(self.hl(), self.h);
                self.push_history("MOV M, H".to_string());
            }
            0x75 => {
                self.write(self.hl(), self.l);
                self.push_history("MOV M, L".to_string());
            }
            0x76 => {
                self.halt = true;
                self.push_history("HLT".to_string());
            }
            0x77 => {
                self.write(self.hl(), self.a);
                self.push_history("MOV M, A".to_string());
            }
            0x78 => {
                self.a = self.b;
                self.push_history("MOV A, B".to_string());
            }
            0x79 => {
                self.a = self.c;
                self.push_history("MOV A, C".to_string());
            }
            0x7a => {
                self.a = self.d;
                self.push_history("MOV A, D".to_string());
            }
            0x7b => {
                self.a = self.e;
                self.push_history("MOV A, E".to_string());
            }
            0x7c => {
                self.a = self.h;
                self.push_history("MOV A, H".to_string());
            }
            0x7d => {
                self.a = self.l;
                self.push_history("MOV A, L".to_string());
            }
            0x7e => {
                self.a = self.read(self.hl());
                self.push_history("MOV A, M".to_string());
            }
            0x7f => {
                self.a = self.a;
                self.push_history("MOV A, A".to_string());
            }
            0x80 => {
                (self.a, self.cy) = self.a.overflowing_add(self.b);
                flag!(self, self.a);
                self.push_history("ADD B".to_string());
            }
            0x81 => {
                (self.a, self.cy) = self.a.overflowing_add(self.c);
                flag!(self, self.a);
                self.push_history("ADD C".to_string());
            }
            0x82 => {
                (self.a, self.cy) = self.a.overflowing_add(self.d);
                flag!(self, self.a);
                self.push_history("ADD D".to_string());
            }
            0x83 => {
                (self.a, self.cy) = self.a.overflowing_add(self.e);
                flag!(self, self.a);
                self.push_history("ADD E".to_string());
            }
            0x84 => {
                (self.a, self.cy) = self.a.overflowing_add(self.h);
                flag!(self, self.a);
                self.push_history("ADD H".to_string());
            }
            0x85 => {
                (self.a, self.cy) = self.a.overflowing_add(self.l);
                flag!(self, self.a);
                self.push_history("ADD L".to_string());
            }
            0x86 => {
                let value = self.read(self.hl());
                (self.a, self.cy) = self.a.overflowing_add(value);
                flag!(self, self.a);
                self.push_history("ADD M".to_string());
            }
            0x87 => {
                (self.a, self.cy) = self.a.overflowing_add(self.a);
                flag!(self, self.a);
                self.push_history("ADD A".to_string());
            }
            0x88 => {
                (self.a, self.cy) = self.a.overflowing_add(self.b.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC B".to_string());
            }
            0x89 => {
                (self.a, self.cy) = self.a.overflowing_add(self.c.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC C".to_string());
            }
            0x8a => {
                (self.a, self.cy) = self.a.overflowing_add(self.d.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC D".to_string());
            }
            0x8b => {
                (self.a, self.cy) = self.a.overflowing_add(self.e.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC E".to_string());
            }
            0x8c => {
                (self.a, self.cy) = self.a.overflowing_add(self.h.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC H".to_string());
            }
            0x8d => {
                (self.a, self.cy) = self.a.overflowing_add(self.l.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC L".to_string());
            }
            0x8e => {
                let value = self.read(self.hl());
                (self.a, self.cy) = self.a.overflowing_add(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC M".to_string());
            }
            0x8f => {
                (self.a, self.cy) = self.a.overflowing_add(self.a.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("ADC A".to_string());
            }
            0x90 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.b);
                flag!(self, self.a);
                self.push_history("SUB B".to_string());
            }
            0x91 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.c);
                flag!(self, self.a);
                self.push_history("SUB C".to_string());
            }
            0x92 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.d);
                flag!(self, self.a);
                self.push_history("SUB D".to_string());
            }
            0x93 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.e);
                flag!(self, self.a);
                self.push_history("SUB E".to_string());
            }
            0x94 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.h);
                flag!(self, self.a);
                self.push_history("SUB H".to_string());
            }
            0x95 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.l);
                flag!(self, self.a);
                self.push_history("SUB L".to_string());
            }
            0x96 => {
                let value = self.read(self.hl());
                (self.a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, self.a);
                self.push_history("SUB M".to_string());
            }
            0x97 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.a);
                flag!(self, self.a);
                self.push_history("SUB A".to_string());
            }
            0x98 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.b.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB B".to_string());
            }
            0x99 => {
                (self.a, self.cy) = self.a.overflowing_sub(self.c.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB C".to_string());
            }
            0x9a => {
                (self.a, self.cy) = self.a.overflowing_sub(self.d.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB D".to_string());
            }
            0x9b => {
                (self.a, self.cy) = self.a.overflowing_sub(self.e.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB E".to_string());
            }
            0x9c => {
                (self.a, self.cy) = self.a.overflowing_sub(self.h.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB H".to_string());
            }
            0x9d => {
                (self.a, self.cy) = self.a.overflowing_sub(self.l.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB L".to_string());
            }
            0x9e => {
                let value = self.read(self.hl());
                (self.a, self.cy) = self.a.overflowing_sub(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB M".to_string());
            }
            0x9f => {
                (self.a, self.cy) = self.a.overflowing_sub(self.a.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.push_history("SBB A".to_string());
            }
            0xa0 => {
                self.a &= self.b;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA B".to_string());
            }
            0xa1 => {
                self.a &= self.c;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA C".to_string());
            }
            0xa2 => {
                self.a &= self.d;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA D".to_string());
            }
            0xa3 => {
                self.a &= self.e;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA E".to_string());
            }
            0xa4 => {
                self.a &= self.h;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA H".to_string());
            }
            0xa5 => {
                self.a &= self.l;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA L".to_string());
            }
            0xa6 => {
                let value = self.read(self.hl());
                self.a &= value;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA M".to_string());
            }
            0xa7 => {
                self.a &= self.a;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ANA A".to_string());
            }
            0xa8 => {
                self.a ^= self.b;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA B".to_string());
            }
            0xa9 => {
                self.a ^= self.c;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA C".to_string());
            }
            0xaa => {
                self.a ^= self.d;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA D".to_string());
            }
            0xab => {
                self.a ^= self.e;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA E".to_string());
            }
            0xac => {
                self.a ^= self.h;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA H".to_string());
            }
            0xad => {
                self.a ^= self.l;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA L".to_string());
            }
            0xae => {
                let value = self.read(self.hl());
                self.a ^= value;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA M".to_string());
            }
            0xaf => {
                self.a ^= self.a;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("XRA A".to_string());
            }
            0xb0 => {
                self.a |= self.b;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA B".to_string());
            }
            0xb1 => {
                self.a |= self.c;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA C".to_string());
            }
            0xb2 => {
                self.a |= self.d;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA D".to_string());
            }
            0xb3 => {
                self.a |= self.e;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA E".to_string());
            }
            0xb4 => {
                self.a |= self.h;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA H".to_string());
            }
            0xb5 => {
                self.a |= self.l;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA L".to_string());
            }
            0xb6 => {
                let value = self.read(self.hl());
                self.a |= value;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA M".to_string());
            }
            0xb7 => {
                self.a |= self.a;
                flag!(self, self.a);
                self.cy = false;
                self.push_history("ORA A".to_string());
            }
            0xb8 => {
                // CMP: compare only sets flags, does NOT modify accumulator
                let (result, borrow) = self.a.overflowing_sub(self.b);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP B".to_string());
            }
            0xb9 => {
                let (result, borrow) = self.a.overflowing_sub(self.c);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP C".to_string());
            }
            0xba => {
                let (result, borrow) = self.a.overflowing_sub(self.d);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP D".to_string());
            }
            0xbb => {
                let (result, borrow) = self.a.overflowing_sub(self.e);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP E".to_string());
            }
            0xbc => {
                let (result, borrow) = self.a.overflowing_sub(self.h);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP H".to_string());
            }
            0xbd => {
                let (result, borrow) = self.a.overflowing_sub(self.l);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP L".to_string());
            }
            0xbe => {
                let value = self.read(self.hl());
                let (result, borrow) = self.a.overflowing_sub(value);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP M".to_string());
            }
            0xbf => {
                // CMP A with itself: result is always 0, no borrow
                let (result, borrow) = self.a.overflowing_sub(self.a);
                self.cy = borrow;
                flag!(self, result);
                self.push_history("CMP A".to_string());
            }
            0xc0 => {
                if !self.z {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RNZ".to_string());
            }
            0xc1 => {
                let bc = self.pop();
                self.set_bc(bc);
                self.push_history("POP B".to_string());
            }
            0xc2 => {
                let addr = self.next_memory();
                self.pc = match self.z {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JNZ {:#06x}", addr));
            }
            0xc3 => {
                let addr = self.next_memory();
                self.pc = addr.wrapping_sub(1);
                self.push_history(format!("JMP {:#06x}", addr));
            }
            0xc4 => {
                let addr = self.next_memory();
                if !self.z {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CNZ {:#06x}", addr));
            }
            0xc5 => {
                self.push(self.bc());
                self.push_history("PUSH B".to_string());
            }
            0xc6 => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_add(value);
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("ADI {:#04x}", value));
            }
            0xc7 => {
                self.rst(0x00);
                self.push_history("RST 0".to_string());
            }
            0xc8 => {
                if self.z {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RZ".to_string());
            }
            0xc9 => {
                self.pc = self.pop().wrapping_sub(1);
                self.push_history("RET".to_string());
            }
            0xca => {
                let addr = self.next_memory();
                self.pc = match self.z {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JZ {:#06x}", addr));
            }
            0xcb => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xcc => {
                let addr = self.next_memory();
                if self.z {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CZ {:#06x}", addr));
            }
            0xcd => {
                let addr = self.next_memory();
                self.call(addr);
                self.push_history(format!("CALL {:#06x}", addr));
            }
            0xce => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_add(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("ACI {:#04x}", value));
            }
            0xcf => {
                self.rst(0x08);
                self.push_history("RST 1".to_string());
            }
            0xd0 => {
                if !self.cy {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RNC".to_string());
            }
            0xd1 => {
                let de = self.pop();
                self.set_de(de);
                self.push_history("POP D".to_string());
            }
            0xd2 => {
                let addr = self.next_memory();
                self.pc = match self.cy {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JNC {:#06x}", addr));
            }
            0xd3 => {
                let port = self.read(self.pc + 1);
                io.port_out(port, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("OUT {:#04x}", port));
            }
            0xd4 => {
                let addr = self.next_memory();
                if !self.cy {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CNC {:#06x}", addr));
            }
            0xd5 => {
                self.push(self.de());
                self.push_history("PUSH D".to_string());
            }
            0xd6 => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("SUI {:#04x}", value));
            }
            0xd7 => {
                self.rst(0x10);
                self.push_history("RST 2".to_string());
            }
            0xd8 => {
                if self.cy {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RC".to_string());
            }
            0xd9 => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xda => {
                let addr = self.next_memory();
                self.pc = match self.cy {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JC {:#06x}", addr));
            }
            0xdb => {
                let port = self.read(self.pc + 1);
                self.a = io.port_in(port);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("IN {:#04x}", port));
            }
            0xdc => {
                let addr = self.next_memory();
                if self.cy {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CC {:#06x}", addr));
            }
            0xdd => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xde => {
                let value = self.read(self.pc + 1);
                (self.a, self.cy) = self.a.overflowing_sub(value.wrapping_add(self.cy as u8));
                flag!(self, self.a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("SBI {:#04x}", value));
            }
            0xdf => {
                self.rst(0x18);
                self.push_history("RST 3".to_string());
            }
            0xe0 => {
                if !self.p {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RPO".to_string());
            }
            0xe1 => {
                let hl = self.pop();
                self.set_hl(hl);
                self.push_history("POP H".to_string());
            }
            0xe2 => {
                let addr = self.next_memory();
                self.pc = match self.p {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JPO {:#06x}", addr));
            }
            0xe3 => {
                // XTHL: Exchange HL with top of stack (SP unchanged)
                let low = self.read(self.sp);
                let high = self.read(self.sp + 1);
                self.write(self.sp, self.l);
                self.write(self.sp + 1, self.h);
                self.l = low;
                self.h = high;
                self.push_history("XTHL".to_string());
            }
            0xe4 => {
                let addr = self.next_memory();
                if !self.p {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CPO {:#06x}", addr));
            }
            0xe5 => {
                self.push(self.hl());
                self.push_history("PUSH H".to_string());
            }
            0xe6 => {
                let value = self.read(self.pc + 1);
                self.a &= value;
                flag!(self, self.a);
                self.cy = false;
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("ANI {:#04x}", value));
            }
            0xe7 => {
                self.rst(0x20);
                self.push_history("RST 4".to_string());
            }
            0xe8 => {
                if self.p {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RPE".to_string());
            }
            0xe9 => {
                // PCHL: Jump to address in HL (subtract 1 because step() adds 1)
                self.pc = self.hl().wrapping_sub(1);
                self.push_history("PCHL".to_string());
            }
            0xea => {
                let addr = self.next_memory();
                self.pc = match self.p {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JPE {:#06x}", addr));
            }
            0xeb => {
                let de = self.de();
                self.set_de(self.hl());
                self.set_hl(de);
                self.push_history("XCHG".to_string());
            }
            0xec => {
                let addr = self.next_memory();
                if self.p {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CPE {:#06x}", addr));
            }
            0xed => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xee => {
                let value = self.read(self.pc + 1);
                self.a ^= value;
                flag!(self, self.a);
                self.cy = false;
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("XRI {:#04x}", value));
            }
            0xef => {
                self.rst(0x28);
                self.push_history("RST 5".to_string());
            }
            0xf0 => {
                if !self.s {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RP".to_string());
            }
            0xf1 => {
                // POP PSW: flags from SP, A from SP+1
                let value = self.pop();
                let flags = (value & 0xFF) as u8;
                self.a = (value >> 8) as u8;
                self.s = flags & (1 << 7) != 0;
                self.z = flags & (1 << 6) != 0;
                self.ac = flags & (1 << 4) != 0;
                self.p = flags & (1 << 2) != 0;
                self.cy = flags & 1 != 0;
                self.push_history("POP PSW".to_string());
            }
            0xf2 => {
                let addr = self.next_memory();
                self.pc = match self.s {
                    false => addr.wrapping_sub(1),
                    true => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JP {:#06x}", addr));
            }
            0xf3 => {
                self.interrupt = false;
                self.push_history("DI".to_string());
            }
            0xf4 => {
                let addr = self.next_memory();
                if !self.s {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CP {:#06x}", addr));
            }
            0xf5 => {
                // PUSH PSW: push A first (to SP-1), then flags (to SP-2)
                // Result: flags at SP, A at SP+1
                let mut flags: u8 = 0x02; // Bit 1 is always set on 8080
                flags |= (self.s as u8) << 7;
                flags |= (self.z as u8) << 6;
                flags |= (self.ac as u8) << 4;
                flags |= (self.p as u8) << 2;
                flags |= self.cy as u8;
                let psw = ((self.a as u16) << 8) | (flags as u16);
                self.push(psw);
                self.push_history("PUSH PSW".to_string());
            }
            0xf6 => {
                let value = self.read(self.pc + 1);
                self.a |= value;
                flag!(self, self.a);
                self.cy = false;
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("ORI {:#04x}", value));
            }
            0xf7 => {
                self.rst(0x30);
                self.push_history("RST 6".to_string());
            }
            0xf8 => {
                if self.s {
                    self.pc = self.pop().wrapping_sub(1);
                }
                self.push_history("RM".to_string());
            }
            0xf9 => {
                self.sp = self.hl();
                self.push_history("SPHL".to_string());
            }
            0xfa => {
                let addr = self.next_memory();
                self.pc = match self.s {
                    true => addr.wrapping_sub(1),
                    false => self.pc.wrapping_add(2),
                };
                self.push_history(format!("JM {:#06x}", addr));
            }
            0xfb => {
                self.interrupt = true;
                self.push_history("EI".to_string());
            }
            0xfc => {
                let addr = self.next_memory();
                if self.s {
                    self.call(addr);
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
                self.push_history(format!("CM {:#06x}", addr));
            }
            0xfd => self
                .history
                .push_back(format!("Invalid: {:#04x}", self.read(self.pc))),
            0xfe => {
                let value = self.read(self.pc + 1);
                let mut a = 0;
                (a, self.cy) = self.a.overflowing_sub(value);
                flag!(self, a);
                self.pc = self.pc.wrapping_add(1);
                self.push_history(format!("CPI {:#04x}", value));
            }
            0xff => {
                self.rst(0x38);
                self.push_history("RST 7".to_string());
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
