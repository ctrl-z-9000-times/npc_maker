//! The NPC Maker is a framework for interacting with simulated environments
//! that contain AI agents. It defines software interfaces that separate the
//! environments from their surrounding concerns, and provides APIs for using them.

pub mod ctrl;
pub mod env;
pub mod evo;

fn read_bytes(reader: &mut impl std::io::BufRead, len: usize) -> std::io::Result<Box<[u8]>> {
    use std::mem::{transmute, MaybeUninit};
    let mut data = unsafe { transmute::<Vec<MaybeUninit<u8>>, Vec<u8>>(vec![MaybeUninit::uninit(); len]) };
    reader.read_exact(&mut data)?;
    Ok(data.into())
}
