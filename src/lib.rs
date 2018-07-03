
extern crate serde;
extern crate integer_encoding;
use std::io::{self,Write,Read,ErrorKind};
use integer_encoding::{VarInt,VarIntWriter,VarIntReader};



fn size_buffer_to(v: &mut Vec<u8>, size: usize) {
	while v.len() < size {
		v.push(0u8);
	}
}

pub struct Bufferer {
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
	pub fn try_read_preambled<R>(&mut self, mut r: R) ->
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

pub fn write_preambled<W>(mut w: W, bytes: &[u8]) -> io::Result<()> where W: Write + Sized {
	w.write_varint(bytes.len())?;
	w.write_all(bytes)
}

pub struct ReadWrapper<R> where R: Read + Sized {
	r: R,
	bufferer: Bufferer,
}

impl<R> ReadWrapper<R> where R: Read + Sized {
	pub fn new(r: R) -> Self {
		Self {
			r,
			bufferer: Bufferer::new(),
		}
	}
	pub fn try_read_preambled(&mut self) -> Result<Option<&mut [u8]>, io::Error> {
		// let (r, b) = (self.r, self.bufferer);
		self.bufferer.try_read_preambled(&mut self.r)
	} 
}

////////////////////////// DEV ////////////////////

#[cfg(test)] 
mod tests;
#[cfg(test)] 
extern crate bincode;
#[cfg(test)] 
#[macro_use]
extern crate serde_derive;
#[cfg(test)] 
#[macro_use]
extern crate lazy_static;


use std::sync::mpsc;
pub struct SendChannel(mpsc::Sender<u8>);
pub struct RecvChannel(mpsc::Receiver<u8>);

pub fn rw_channel() -> (SendChannel, RecvChannel) {
	let (a, b) = mpsc::channel();
	(SendChannel(a), RecvChannel(b))
}

impl io::Write for SendChannel {
	fn write(&mut self, bytes: &[u8]) -> Result<usize, io::Error> {
		for byte in bytes.iter().cloned() {
			if let Err(_) = self.0.send(byte) {
				return Err(io::ErrorKind::UnexpectedEof.into())
			}
		}
		Ok(bytes.len())
	}
    fn flush(&mut self) -> Result<(), io::Error> {
    	Ok(())
    }
}

impl io::Read for RecvChannel {
	fn read(&mut self, buffer: &mut [u8]) -> Result<usize, io::Error> {
		for i in 0..buffer.len() {
			match self.0.recv() {
				Ok(x) => buffer[i] = x,
				Err(_) => return Err(io::ErrorKind::UnexpectedEof.into()),
			};
		}
		Ok(buffer.len())
	}
}