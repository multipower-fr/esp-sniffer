#[macro_use]
extern crate lazy_static;
// extern crate itertools;

use std::collections::HashMap;
use std::io::{self, Read};
use std::str;
use std::sync::Mutex;
use std::time::Duration;

use serialport;
use itertools::Itertools;

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

fn main() {
    let mut port = serialport::new("\\\\.\\COM3", 115200)
        .timeout(Duration::from_millis(20))
        .open_native()
        .expect("Failed to open port");
    let mut serial_buf: Vec<u8> = vec![0; 1000];
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => parse(serial_buf.to_vec(), t),
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

/*
fn convert_to_mac(spliced: &[u8]) -> String {
    let mut mac: String = String::from("");

    return mac
}
*/

fn parse(frame: Vec<u8>, t: usize) {
    // io::stdout().write_all(&frame[..t]).unwrap();
    let channel: u8;
    let channel_nums: &[u8];
    let mac_address: &str;
    let rssi: &str;
    let ssid: &str;

    let sep_count = frame.clone().into_iter().filter(|&sep| sep == 31).count();
    if frame[2] == 0x1F && frame[0] == 0x02 {
        channel = frame[1] - 48;
        // 20: Position du dernier char de l'adresse + 1
        mac_address = str::from_utf8(&frame[3..20]).unwrap();
        rssi = str::from_utf8(&frame[21..24]).unwrap();
        if sep_count > 3 {
            let ssid_splice = &frame[25..];
            let ssid_split: Vec<&str>;
            match str::from_utf8(&ssid_splice[..]) {
                Ok(t) => ssid_split = t.split('\x1F').take(1).collect(),
                Err(_) => ssid_split = vec![""],
            }
            ssid = ssid_split[0]
        } else {
            ssid = "";
        }
    } else if frame[0] == 0x02 {
        channel_nums = &frame[1..3];
        // Conversion ASCII -> Nombre
        channel = ((channel_nums[0] - 48) * 10) + (channel_nums[1] - 48);
        // 21: Position du dernier char de l'adresse + 1
        mac_address = str::from_utf8(&frame[4..21]).unwrap();
        rssi = str::from_utf8(&frame[22..25]).unwrap();
        if sep_count > 3 {
            let ssid_splice = &frame[26..];
            let ssid_split: Vec<&str>;
            match str::from_utf8(&ssid_splice[..]) {
                Ok(t) => ssid_split = t.split('\x1F').take(1).collect(),
                Err(_) => ssid_split = vec![""],
            }
            ssid = ssid_split[0]
        } else {
            ssid = "";
        }
    } else {
        channel = 0;
        mac_address = "";
        rssi = "";
        ssid = "";
    }
    println!("{channel} {mac_address} {rssi} {ssid}");
    process(
        channel,
        mac_address.to_string(),
        rssi.to_string(),
        ssid.to_string(),
    )
}

fn process(channel: u8, mac_address: String, rssi: String, ssid: String) {
    let mut channel_table = CHANNELS.lock().unwrap();
    let mut ssid_table = SSIDS.lock().unwrap();
    let mut rssi_table = RSSIS.lock().unwrap();
    channel_table
        .entry(mac_address.clone())
        .or_insert_with(Vec::new)
        .push(channel);
    ssid_table
        .entry(mac_address.clone())
        .or_insert_with(Vec::new)
        .push(ssid);
    rssi_table
        .entry(mac_address.clone())
        .or_insert_with(Vec::new)
        .push(rssi);
    dedupe()
}

fn dedupe() {
    let mut channel_table = CHANNELS.lock().unwrap();
    let mut ssid_table = SSIDS.lock().unwrap();
    let mut rssi_table = RSSIS.lock().unwrap();


}
