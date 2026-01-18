# Intel 8080 Emulator

A cycle-accurate Intel 8080 CPU emulator running Space Invaders in the browser.

**Play it:** https://8080.mus.sh

![Space Invaders running on the Intel 8080 emulator](og-image.png)

## Features

- Full Intel 8080 instruction set emulation
- Cycle-accurate timing with interrupt handling (RST 1 at mid-screen, RST 2 at vblank)
- Space Invaders hardware: shift register, input ports, 1-bit video output
- Debug panel showing CPU state, disassembly, and memory view
- Mobile touch controls

## Build

Requires Rust and the `wasm32-unknown-unknown` target.

```bash
rustup target add wasm32-unknown-unknown

# Build for web
./build-web.sh

# Run locally
python3 -m http.server 8080
```

## Deploy

```bash
./deploy-cloudflare.sh
npx wrangler pages deploy dist --project-name=intel-8080-emu
```

## Controls

| Key | Action |
|-----|--------|
| C | Insert coin |
| 1 / 2 | Start 1P / 2P |
| A/D or Arrows | Move |
| Space or W | Fire |
| P | Pause |
| N | Step (when paused) |
| R | Reset |

## Architecture

The emulator runs at 2MHz (the original 8080 clock speed). The Space Invaders hardware generates two interrupts per frame at 60Hz:
- RST 1 (0xCF) at mid-screen (~96 scanlines)
- RST 2 (0xD7) at vblank (~224 scanlines)

Video RAM at 0x2400-0x3FFF is rendered as a 256x224 1-bit framebuffer, rotated 90° CCW to match the original arcade cabinet orientation.

## Tech

- Rust + WebAssembly
- [Macroquad](https://macroquad.rs/) for graphics/input
- Cloudflare Pages for hosting
