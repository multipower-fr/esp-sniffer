#[macro_use]
extern crate lazy_static;
extern crate crossbeam;

use std::collections::HashMap;
use std::io::{self, Read};
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crossbeam::queue::SegQueue;
use serialport;

lazy_static! {
    static ref CHANNELS: Mutex<HashMap<String, Vec<u8>>> = {
        let mut cha = HashMap::new();
        Mutex::new(cha)
    };
    static ref SSIDS: Mutex<HashMap<String, Vec<String>>> = {
        let mut s = HashMap::new();
        Mutex::new(s)
    };
    static ref RSSIS: Mutex<HashMap<String, Vec<String>>> = {
        let mut r = HashMap::new();
        Mutex::new(r)
    };
}

fn push_to_queue(q: Arc<SegQueue<Vec<u8>>>, frame: Vec<u8>) -> thread::JoinHandle<()> {
    thread::spawn(move || q.push(frame.clone()))
}

fn run_consumer(q: Arc<SegQueue<Vec<u8>>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let frame: Option<Vec<u8>> = q.pop();
            match frame {
                Some(x) => println!("{:?}", x),
                None => (),
            };
        }
    })
}

static NTHREADS: u32 = 3;

fn main() {
    let queue: SegQueue<Vec<u8>> = SegQueue::new();
    let arc_queue = Arc::new(queue);
    let mut thread_handles: Vec<thread::JoinHandle<()>> = Vec::new();

    let mut port = serialport::new("\\\\.\\COM3", 115_200)
        .timeout(Duration::from_millis(20))
        .open_native()
        .expect("Failed to open port");
    for _ in 0..NTHREADS {
        let cloned_queue = arc_queue.clone();
        run_consumer(cloned_queue);
    }
    let mut serial_buf: Vec<u8> = vec![0; 1000];
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(_) => thread_handles.push(push_to_queue(arc_queue.clone(), serial_buf.to_vec())),
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn parse(frame: Option<Vec<u8>>) {
    // io::stdout().write_all(&frame[..t]).unwrap();
    let channel: u8;
    let mac_address: &str;
    let rssi: &str;
    let ssid: &str;
    let str_frame: &str;
}
