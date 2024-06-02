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

