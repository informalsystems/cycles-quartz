use std::{
    fs::File,
    io::{self, Read, Write},
};

fn main() -> io::Result<()> {
    let user_data = [0u8; 64];
    let mut user_report_data = File::create("/dev/attestation/user_report_data")?;
    user_report_data.write_all(user_data.as_slice())?;
    user_report_data.flush()?;

    let mut file = File::open("/dev/attestation/quote")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let quote_hex = hex::encode(&buffer);
    print!("{}", quote_hex);

    Ok(())
}
