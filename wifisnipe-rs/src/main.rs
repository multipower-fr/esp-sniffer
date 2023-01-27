#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::io::{self, Read};
use std::str;
use std::sync::Mutex;
use std::time::Duration;

use serialport;
use itertools::Itertools;
use regex::Regex;

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
    let str_frame: &str;
    let tmp_frame = &frame[1..t-2];


    match str::from_utf8(&tmp_frame) {
        Ok(r) => str_frame = r,
        Err(_) => str_frame = ""
    }

    let mut str_frame_split: Vec<&str> = str_frame.split('\x1F').collect_vec();
    channel = str_frame_split[0];
    mac_address = str_frame_split[1];
    ssid

    // let sep_count = tmp_frame.clone().into_iter().filter(|&sep| *sep == 31).count();

    /*
    if tmp_frame[2] == 0x1F && tmp_frame[0] == 0x02 {
        channel = tmp_frame[1] - 48;
        // 20: Position du dernier char de l'adresse + 1
        mac_address = str::from_utf8(&tmp_frame[3..20]).unwrap();
        rssi = str::from_utf8(&tmp_frame[21..24]).unwrap();
        if sep_count > 3 {
            let ssid_splice = &tmp_frame[25..];
            let ssid_split: Vec<&str>;
            match str::from_utf8(&ssid_splice[..]) {
                Ok(t) => ssid_split = t.split('\x1F').take(1).collect(),
                Err(_) => ssid_split = vec![""],
            }
            ssid = ssid_split[0]
        } else {
            ssid = "";
        }
    } else if tmp_frame[0] == 0x02 {
        channel_nums = &tmp_frame[1..3];
        // Conversion ASCII -> Nombre
        channel = ((channel_nums[0] - 48) * 10) + (channel_nums[1] - 48);
        // 21: Position du dernier char de l'adresse + 1
        mac_address = str::from_utf8(&tmp_frame[4..21]).unwrap();
        rssi = str::from_utf8(&tmp_frame[22..25]).unwrap();
        if sep_count > 3 {
            let ssid_splice = &tmp_frame[26..];
            let ssid_split: Vec<&str>;
            match str::from_utf8(&ssid_splice[..]) {
                Ok(t) => ssid_split = t.split('\x1F').take(1).collect(),
                Err(_) => ssid_split = vec![""],
            }
            lazy_static!  {
                static ref RE: Regex = Regex::new("[[:cntrl:]]").unwrap();
            }
            if ! RE.is_match(ssid_split[0]) {
                ssid = ""
            }
            else {
                ssid = ssid_split[0]
            }
        } else {
            ssid = "";
        }
    } else {
        channel = 0;
        mac_address = "";
        rssi = "";
        ssid = "";
    }*/
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
    drop(channel_table);
    drop(ssid_table);
    drop(rssi_table);
    dedupe(mac_address)
}

fn dedupe(mac_address: String) {
    // Get the global singleton
    let mut channel_table = CHANNELS.lock().unwrap();
    let mut ssid_table = SSIDS.lock().unwrap();
    let mut rssi_table = RSSIS.lock().unwrap();
    
    // De-duplicate vector in place
    let tmp_channel_table = channel_table.get(&mac_address.clone())
        .clone();
    let tmp_channel_dedupe: Vec<_> = tmp_channel_table
        .into_iter()
        .flatten()
        .unique()
        .cloned()
        .collect();
    channel_table.insert(mac_address.clone(), tmp_channel_dedupe);

    let tmp_ssid_table = ssid_table.get(&mac_address.clone())
        .clone();
    let tmp_ssid_dedupe: Vec<_> = tmp_ssid_table
        .into_iter()
        .flatten()
        .unique()
        .cloned()
        .collect();
    ssid_table.insert(mac_address.clone(), tmp_ssid_dedupe);

    let tmp_rssi_table = rssi_table.get(&mac_address.clone())
        .clone();
    let tmp_rssi_dedupe: Vec<_> = tmp_rssi_table
        .into_iter()
        .flatten()
        .unique()
        .cloned()
        .collect();
    rssi_table.insert(mac_address.clone(), tmp_rssi_dedupe);

    // println!("{:} {:?} {:?} {:?}", mac_address, channel_table.get(&mac_address), ssid_table.get(&mac_address), rssi_table.get(&mac_address));
}
