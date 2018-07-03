use super::*;

use std::net::{TcpStream,TcpListener};
use std::thread;


#[test]
fn it_works() {
	let [mut a, mut b] = tcp_pipe();
	let x = write_into(&mut a, &[1,2,3]).unwrap();
	println!("wrote {:?}", x);


	let x = write_into(&mut a, &[6,6,6,6,6,6,123,132,31,2,132,321,123,12,123]).unwrap();
	println!("wrote {:?}", x);

	let mut r = Bufferer::new();
	// let mut buf = [0u8; 64];
	for x in 0..10 {
		println!("-------------");
		let x = r.read_from(&mut b);
		println!("{:?}", x);
	}
}


fn tcp_pipe() -> [TcpStream; 2] {
	for port in 200..=std::u16::MAX {
		let addr = format!("127.0.0.1:{}", port);
		if let Ok(listener) = TcpListener::bind(&addr) {
			let handle = thread::spawn(move || {
				let x = listener.accept().unwrap().0;
				x.set_nonblocking(true).unwrap();
				x
			});
			let y = TcpStream::connect(&addr).unwrap();
			y.set_nonblocking(true).unwrap();
			return [
				y,
				handle.join().unwrap(),
			];
		}
	}
	panic!("NO PORTS LEFT!")
}
