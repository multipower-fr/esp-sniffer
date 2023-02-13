#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::io::{self, Read};
use std::mem::MaybeUninit;
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ringbuf::{Consumer, HeapRb, SharedRb};

lazy_static! {
    static ref CHANNELS: Mutex<HashMap<String, Vec<u8>>> = {
        let cha = HashMap::new();
        Mutex::new(cha)
    };
    static ref SSIDS: Mutex<HashMap<String, Vec<String>>> = {
        let s = HashMap::new();
        Mutex::new(s)
    };
    static ref RSSIS: Mutex<HashMap<String, Vec<String>>> = {
        let r = HashMap::new();
        Mutex::new(r)
    };
}

fn main() {
    // FIFO queue
    let rb = HeapRb::<Vec<u8>>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut tx, rx) = rb.split();
    // Envoi du consommateur dans le thread pour le traitement
    thread::spawn(move || {
        parse(rx);
    });
    // Ouvrir le port serie
    let mut port = serialport::new("\\\\.\\COM3", 115_200)
        .timeout(Duration::from_millis(30))
        .open_native()
        .expect("Failed to open port");
    // Buffer serie
    let mut serial_buf: Vec<u8> = vec![0; 1000];
    // Cache la valeur de retour du send
    let mut _res;
    // TODO: Revoir ce qu'est
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(_) => _res = tx.push(serial_buf.to_vec()),
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn parse(mut queue_rx: Consumer<Vec<u8>, Arc<SharedRb<Vec<u8>, Vec<MaybeUninit<Vec<u8>>>>>>) {
    loop {
        while queue_rx.is_empty() {
            // Ne rien faire si la queue est vide
        }
        // Recupere un element de la FIFO
        let frame: Vec<u8> = queue_rx.pop().unwrap();
        let mut frame_tmp: Vec<u8> = frame.clone();
        // TODO: Faire le découpage
        // Enleve le cractere de debut de transmission
        frame_tmp.remove(0);
        /* if frame_tmp[frame_tmp.len()-1] == 0x1F {
            frame_tmp.remove(frame_tmp.len() -1);
        }*/
        // Convertis le vecteur de codes ASCII en chaine de caracteres
        let str_frame: &str = match str::from_utf8(&frame_tmp) {
            Ok(f) => f,
            Err(_) => "ERROR",
        };
        println!("{str_frame}");

        // Split au niveau des caracteres de controle
        let splitted_frame: Vec<_> = str_frame.split('\x1F').collect();
        // println!("{:?}", splitted_frame);
        // Check suivant si le channel est a un chiffre ou a deux chiffres
        let channel: u8 = if frame_tmp[1] == 0x1F {
            // Enlever 48 pour retourner du code ASCII au numéro
            frame_tmp[0] - 48
        } else {
            (splitted_frame[0]).parse::<u8>().unwrap()
        };
        // println!("{:}", channel);
        let mac_address: &str = splitted_frame[1];
        let rssi: &str = splitted_frame[2];
        let ssid: &str = splitted_frame[splitted_frame.len() - 1];
        println!("{channel} {mac_address} {rssi} {ssid}");
        // TODO: Ajouter aux HashMaps
    }
}
