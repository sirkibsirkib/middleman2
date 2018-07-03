use super::*;

use std::net::{TcpStream,TcpListener};
use std::thread;
use std::io::BufWriter;
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
fn pipe_raw_bytes() {
	let (a, mut b) = rw_channel();
	let mut a = BufWriter::new(a);

	// write
	for msg in RAW_BYTES.iter() {
		write_preambled(&mut a, msg).unwrap();
	}
	a.flush().unwrap();

	// thread::sleep(SLEEPTIME);

	// read
	let mut r = Bufferer::new();
	for msg in RAW_BYTES.iter() {
		let x = r.try_read_preambled(&mut b).unwrap().unwrap();
		assert_eq!(msg, &x);
	}
}

#[test]
fn pipe_raw_bytes_wrapped() {
	let (a, b) = rw_channel();
	let (mut a, mut r) = (BufWriter::new(a), ReadWrapper::new(b));

	// write
	for msg in RAW_BYTES.iter() {
		write_preambled(&mut a, msg).unwrap();
	}
	a.flush().unwrap();

	// thread::sleep(SLEEPTIME);

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



////////////////////// AUX /////////////////////////

fn prep(stream: &TcpStream) {
	stream.set_nonblocking(true).unwrap();
	stream.set_nodelay(true).unwrap();
}


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
