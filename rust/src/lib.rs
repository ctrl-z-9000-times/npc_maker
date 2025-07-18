//! The NPC Maker is a framework for interacting with simulated environments
//! which contain AI agents. It provides a set of interfaces which separate the
//! environments from their surrounding concerns, and APIs for using the interfaces.

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
