use super::*;

// use std::net::{TcpStream,TcpListener};
use std::thread;
use std::io::BufWriter;
use std::io::Read;
use std::io::ErrorKind;
use std::time;

lazy_static! {
    static ref RAW_BYTES: Vec<Vec<u8>> = {
        vec![
        	vec![0,1,3,4],
        	vec![2,3,1],
        	vec![],
        	vec![0,32,34,43,34,34,3,2,23,4],
        ]
    };
}	

const SLEEPTIME: time::Duration = time::Duration::from_millis(500);

#[test]
fn raw() {
	let (a, mut b) = rw_channel();
	let mut a = BufWriter::new(a);

	// write
	for msg in RAW_BYTES.iter() {
		write_preambled(&mut a, msg).unwrap();
	}
	a.flush().unwrap();

	// read
	let mut r = Bufferer::new();
	for msg in RAW_BYTES.iter() {
		let x = r.try_read_preambled(&mut b).unwrap().unwrap();
		assert_eq!(msg, &x);
	}
}

#[test]
fn raw_wrapped() {
	let (a, b) = rw_channel();
	let (mut a, mut r) = (BufWriter::new(a), ReadWrapper::new(b));

	// write
	for msg in RAW_BYTES.iter() {
		write_preambled(&mut a, msg).unwrap();
	}
	a.flush().unwrap();

	// read
	for msg in RAW_BYTES.iter() {
		let x = r.try_read_preambled().unwrap().unwrap();
		assert_eq!(msg, &x);
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Whatever {
	x: u32,
	y: u64,
	z: String,
}

#[test]
fn raw_wrapped_serde() {
	let (a, b) = rw_channel();
	let (mut a, mut r) = (BufWriter::new(a), ReadWrapper::new(b));
	let messages = vec![
		Whatever {x:32, y:243, z:"Hello, there.".into()},
		Whatever {x:23, y:11, z:"Peace, friend.".into()},
		Whatever {x:1231, y:12324, z:"My, you're a tall one!".into()},
		Whatever {x:0, y:23, z:"What would you ask of Death?".into()},
	];

	// write
	for msg in messages.iter() {
		let vec = bincode::serialize(msg).unwrap();
		write_preambled(&mut a, &vec).unwrap();
	}
	a.flush().unwrap();

	// read
	for msg in messages.iter() {
		let msg2 = r.try_read_preambled().unwrap().unwrap();
		let mut slice: &[u8] = &msg2;
		let de = bincode::deserialize_from::<_,Whatever>(&mut slice).unwrap();
		assert_eq!(msg, &de);
	}
}

struct Bincoder;
impl CanSerialize for Bincoder {
	fn serialize_into<T,W>(&mut self, t: &T, w: W) -> Result<(), io::Error> where T: Serialize, W: io::Write {
		match bincode::serialize_into(w, t) {
			Ok(x) => Ok(x),
			Err(e) => Err(ErrorKind::InvalidData.into()),
		}
	}
}
impl CanDeserialize for Bincoder {
	fn deserialize<T>(&mut self, bytes: &[u8]) -> Result<T, io::Error> where T: DeserializeOwned {
		println!("PLEASE DESERIALIZE {:?}", bytes);
		match bincode::deserialize_from::<_,T>(bytes) {
			Ok(t) => Ok(t),
			Err(e) => Err(ErrorKind::InvalidData.into()),
		}
	}
}

#[test]
fn both_wrapped_serde() {
	let (a, b) = rw_channel();
	let (mut w, mut r) = (Ser::new(a, Bincoder), De::new(b, Bincoder));
	let messages = vec![
		Whatever {x:32, y:243, z:"Hello, there.".into()},
		Whatever {x:23, y:11, z:"Peace, friend.".into()},
	];

	// write
	for msg in messages.iter() {
		w.write_msg(msg).unwrap();
	}
	w.flush().unwrap();

	//read
	for msg in messages.iter() {
		let msg2 = r.try_read::<Whatever>().unwrap().unwrap();
		assert_eq!(msg, &msg2);
	}
}



////////////////////// AUX /////////////////////////

// fn prep(stream: &TcpStream) {
// 	stream.set_nonblocking(true).unwrap();
// 	stream.set_nodelay(true).unwrap();
// }


// fn tcp_pipe() -> [TcpStream; 2] {
// 	for port in 200..=std::u16::MAX {
// 		let addr = format!("127.0.0.1:{}", port);
// 		if let Ok(listener) = TcpListener::bind(&addr) {
// 			let handle = thread::spawn(move || {
// 				let x = listener.accept().unwrap().0;
// 				prep(&x);
// 				x
// 			});
// 			let y = TcpStream::connect(&addr).unwrap();
// 			prep(&y);
// 			return [
// 				y,
// 				handle.join().unwrap(),
// 			];
// 		}
// 	}
// 	panic!("NO PORTS LEFT!")
// }
