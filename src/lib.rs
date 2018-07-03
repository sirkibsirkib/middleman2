use std::collections::VecDeque;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate integer_encoding;
// use serde::se::Serializer
use std::io::{self,Read,Write,ErrorKind};
use integer_encoding::{VarInt,VarIntWriter,VarIntReader};

fn size_buffer_to(v: &mut Vec<u8>, size: usize) {
	while v.len() < size {
		v.push(0u8);
	}
}

struct Bufferer {
	buffer: Vec<u8>,
	occupied: usize,
	len: Option<(u64,u8)> // (payload bytes, buffer offset) 
}
impl Bufferer {
	pub fn new() -> Self {
		Self {
			buffer: Vec::new(),
			occupied: 0,
			len: None,
		}
	}
	pub fn try_read_from<R>(&mut self, mut r: R) ->
			Result<Option<&mut [u8]>, io::Error>
	where R: io::Read
	{
		loop {
			// determine how many bytes next read() call should attempt to get
			let read_to = if let Some((payload_bytes, len_bytes)) = self.len {
				// len of payload is known. _that_ number of bytes + len offset
				let t = payload_bytes as usize + len_bytes as usize;
				size_buffer_to(&mut self.buffer, t);
				t
			} else {
				// len isnt known. just 1. proceed cautiously
				let t = self.occupied as usize + 1;
				size_buffer_to(&mut self.buffer, t);
				t
			};
			// read into unoccupied part of buffer
			match r.read(&mut self.buffer[self.occupied..read_to]) {
				Ok(bytes_read) => {
					self.occupied += bytes_read;
					if let Some((payload_bytes, len_bytes)) = self.len {
						// reading payload
						if payload_bytes as usize + len_bytes as usize 	== self.occupied {
							//reset state. return result in-place
							let temp = self.occupied;
							self.occupied = 0;
							self.len = None;
							return Ok(Some(&mut self.buffer[len_bytes as usize..temp]));
						} 
						// else, continue spinning
					} else {
						// reading len preamble
						match (&self.buffer[..self.occupied]).read_varint() {
							Err(e) => {
								if e.kind() == ErrorKind::UnexpectedEof {
									return Ok(None);
								} else {
									println!("varint BROKEN");
									return Err(e);
								}
							},
							Ok(x) => {
								self.len = Some((x, x.required_space() as u8));
							},
						}
					}
				},
				Err(e) => {
					// error attempting to read
					if e.kind() == ErrorKind::WouldBlock {
						// reader isnt broken, just isnt ready.
						return Ok(None);
					} else {
						// reader state is bust. report err.
						return Err(e);
					}
				},
			}
		}
	}
}

pub fn write_into<W>(mut w: W, bytes: &[u8]) -> io::Result<()> where W: Write + Sized {
	w.write_varint(bytes.len())?;
	let b = w.write_all(bytes)?;
}

#[cfg(test)] 
mod tests;