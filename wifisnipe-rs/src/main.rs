#[macro_use]
extern crate lazy_static;

use std::collections::{HashMap, VecDeque};
use std::io::{self, Read};
use std::str;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use actix::prelude::*;
use ringbuf::HeapRb;
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

// Acteur pour la

#[derive(Message)]
#[rtype(result = "()")]
struct Frame(Vec<u8>, usize);

struct Parser;
impl Actor for Parser {
    type Context = Context<Self>;
}
impl Handler<Frame> for Parser {
    type Result = ();
    fn handle(&mut self, msg: Frame, _ctx: &mut Self::Context) -> Self::Result {
        
    }
}

#[actix::main]
async fn main() {
    let mut rb = HeapRb::<Vec<u8>>::new(100);
    let (mut tx, mut rx) = rb.split();
    thread::spawn(move || {
        parse(rx);
    });
    // Lancer l'acteur
    let parser_addr = Parser.start();
    // Ouvrir le port serie
    let mut port = serialport::new("\\\\.\\COM3", 115_200)
        .timeout(Duration::from_millis(30))
        .open_native()
        .expect("Failed to open port");
    // Buffer serie
    let mut serial_buf: Vec<u8> = vec![0; 1000];
    // Cache la valeur de retour du send
    let mut _res;
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => _res = parser_addr.send(Frame(serial_buf.to_vec(), t)).await,
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn parse() {
    while 
    loop {
        let channel: u8;
        let mac_address: &str;
        let rssi: &str;
        let ssid: &str;
        let str_frame: &str;
        let splitted_frame: Vec<_>;
        let mut frame_tmp = frame.clone();

        // Enleve le cractere de debut de transmission
        frame_tmp.remove(0);
        /* if frame_tmp[frame_tmp.len()-1] == 0x1F {
            frame_tmp.remove(frame_tmp.len() -1);
        }*/
        // Convertis le vecteur de codes ASCII en chaine de caracteres
        match str::from_utf8(&frame_tmp) {
            Ok(f) => str_frame = f,
            Err(_) => str_frame = "",
        }
        println!("{str_frame}");
        // Split au niveau des caracteres de controle
        splitted_frame = str_frame.split('\x1F').collect();
        // println!("{:?}", splitted_frame);
        // Check suivant si le channel est a un chiffre ou a deux chiffres
        if frame_tmp[1] == 0x1F {
            channel = frame_tmp[0] - 48;
        } else {
            channel = (splitted_frame[0]).parse::<u8>().unwrap()
        }
        // println!("{:}", channel);
        mac_address = splitted_frame[1];
        rssi = splitted_frame[2];
        ssid = splitted_frame[splitted_frame.len() - 1];
        // println!("{channel} {mac_address} {rssi} {ssid}");
    }
}
