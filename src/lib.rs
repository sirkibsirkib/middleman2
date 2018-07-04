
extern crate serde;
extern crate integer_encoding;
use serde::{Serialize,de::DeserializeOwned};
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
		self.bufferer.try_read_preambled(&mut self.r)
	} 
}

pub trait CanSerialize {
	fn serialize_into<T,W>(&mut self, t: &T, w: W) -> Result<(), io::Error> where T: Serialize, W: io::Write;
}
pub trait CanDeserialize {
	fn deserialize<T>(&mut self, &[u8]) -> Result<T, io::Error> where T: DeserializeOwned;
}

pub struct Ser<S,W> where S: CanSerialize + Sized, W: io::Write + Sized {
	ser: S,
	writer: W,
	buffer: GrowingBuffer,
}

impl<S,W> Ser<S,W> where S: CanSerialize + Sized, W: io::Write + Sized {
	pub fn new(writer:W, ser:S) -> Self {
		Self { writer, ser, buffer: GrowingBuffer::new() }
	}
	pub fn write_msg<T>(&mut self, t:&T) -> Result<usize, io::Error> where T: Serialize {
		self.buffer.clear();
		self.ser.serialize_into(t, &mut self.buffer)?;
		write_preambled(&mut self.writer, self.buffer.contents())?;
		let wrote = self.buffer.occupancy();
		self.buffer.clear();
		Ok(wrote)
	}
	pub fn flush(&mut self) -> Result<(),io::Error> {
		self.writer.flush()
	}
}

struct GrowingBuffer {
	buf: Vec<u8>,
	occupied: usize,
}
impl GrowingBuffer {
	fn new() -> Self {
		Self {buf: vec![], occupied:0}
	}
	fn clear(&mut self) {
		self.occupied = 0;
	}
	fn occupancy(&self) -> usize {
		self.occupied
	}
	fn contents(&self) -> &[u8] {
		& self.buf[0..self.occupied]
	}
}
impl io::Write for GrowingBuffer {
    fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
    	let end = self.occupied + buf.len();
    	size_buffer_to(&mut self.buf, end);
    	(&mut self.buf[self.occupied..end]).write(buf)?;
    	self.occupied += buf.len();
    	Ok(buf.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
    	Ok(())
    }
}

pub struct De<R,D> where R: Read, D: CanDeserialize {
	reader: ReadWrapper<R>,
	holding: Option<*const [u8]>,
	de: D,
}
impl<R,D> De<R,D> where R: io::Read, D: CanDeserialize {
	pub fn new(reader:R, de:D) -> Self {
		Self { reader: ReadWrapper::new(reader), de, holding: None }
	}
	pub fn try_read<T>(&mut self) -> Result<Option<T>, io::Error> where T: DeserializeOwned {
		// 1. read something into the internal buffer if we haven't already
		if self.holding.is_none() {
			if let Some(slice) = self.reader.try_read_preambled()? {
				self.holding = Some(slice as *const [u8]);
			} else {
				return Ok(None);
			}
		}
		let raw_slice  = self.holding.unwrap();
		let t = self.de.deserialize(unsafe{&*raw_slice})?;
		self.holding = None;
		Ok(Some(t))
	}
	pub fn pop_holding(&mut self) -> bool {
		if self.holding.is_some() {
			self.holding = None;
			true
		} else {
			false
		}
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
	#[inline]
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
	#[inline]
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
