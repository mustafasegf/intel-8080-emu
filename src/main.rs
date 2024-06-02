use anyhow::Result;

fn main() -> Result<()> {
    println!("8080 disassembler");

    let rom = std::fs::read("./rom/invaders.h").expect("Unable to read file");

    println!("first 10 byte");
    for byte in &rom[..10] {
        println!("{byte:x}");
    }

    Ok(())
}

#[derive(Debug)]
enum Register {
    B,
    C,
    D,
    E,
    H,
    L,
    M,
    A,
}

#[derive(Debug)]
enum Instruction {
    Unimplemented(String),
    Invalid,
    Nop,
    Lxi { reg: Register, lo: u8, hi: u8 },
    Stax { reg: Register },
    Inx { reg: Register },
    Inr { reg: Register },
    Dcr { reg: Register },
    Mvi { reg: Register, lo: u8 },
    Rlc,
    Dad { reg: Register },
    Ldax { reg: Register },
    Dcx { reg: Register },
    Rrc,
    Ral,
    Rar,
    Shld { lo: u8, hi: u8 },
    Daa,
    Lhld { lo: u8, hi: u8 },
    Cma,
    Sta { lo: u8, hi: u8 },
    Stc,
    Lda { lo: u8, hi: u8 },
    Cmc,
    Mov { from: Register, to: Register },
    Hlt,
    Add { reg: Register, overflow: bool },
    Sbb { reg: Register },
    Ana { reg: Register },
    Xra { reg: Register },
    Ora { reg: Register },
    Cmp { reg: Register },
    Rnz,
    Pop { reg: Register },
    Jnz { lo: u8, hi: u8 },
    Jmp { lo: u8, hi: u8 },
    Cnz { lo: u8, hi: u8 },
    Push { reg: Register },
    Adi { lo: u8 },
    Rst { offset: u8 },
    Rz,
    Ret,
    Jz { lo: u8, hi: u8 },
    Cz { lo: u8, hi: u8 },
    Call { lo: u8, hi: u8 },
    Aci { lo: u8 },
    Rnc,
    Jnc { lo: u8, hi: u8 },
    Out { lo: u8 },
    Cnc { lo: u8, hi: u8 },
    Sui { lo: u8 },
    Rc,
    Jc { lo: u8, hi: u8 },
    In { lo: u8 },
    Cc { lo: u8, hi: u8 },
    Sbi { lo: u8 },
    Rpo,
    Jpo { lo: u8, hi: u8 },
    Xthl,
    Cpo { lo: u8, hi: u8 },
    Ani { lo: u8 },
    Rpe,
    Pchl,
    Jpe { lo: u8, hi: u8 },
    Xchg,
    Cpe { lo: u8, hi: u8 },
    Xri { lo: u8 },
    Rp,
    Jp { lo: u8, hi: u8 },
    Di,
    Cp { lo: u8, hi: u8 },
    Ori { lo: u8 },
    Rm,
    Sphl,
    Jm { lo: u8, hi: u8 },
    Ei,
    Cm { lo: u8, hi: u8 },
    Cpi { lo: u8 },
}
