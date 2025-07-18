//! The NPC Maker is a framework for interacting with simulated environments
//! that contain AI agents. It defines software interfaces that separate the
//! environments from their surrounding concerns, and provides APIs for using them.

pub mod ctrl;
pub mod env;

fn read_bytes(reader: &mut impl std::io::BufRead, len: usize) -> std::io::Result<Box<[u8]>> {
    let mut data = Vec::with_capacity(len);
    unsafe {
        data.set_len(len);
    }
    reader.read_exact(&mut data)?;
    Ok(data.into())
}
