// Jackson Coxson

use std::{
    fs::File,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use log::{error, info, warn, LevelFilter};
use plist_plus::{error::PlistError, Plist};
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};

use crate::{heartbeat::start_beat, raw_packet::RawPacket, Errors};

const LISTEN_PORT: u16 = 27015;

pub fn listen(pairing_file: Plist) {
    std::thread::Builder::new()
        .name("muxer".to_string())
        .spawn(move || {
            // Create the listener
            let mut listener = TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                LISTEN_PORT,
            ))
            .unwrap();
            let mut retries = 0;
            loop {
                // Listen for requests
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(_) => {
                        retries += 1;
                        std::thread::sleep(std::time::Duration::from_millis(5));

                        if retries < 50 {
                            continue;
                        } else {
                            // Rebind
                            warn!("minimuxer is rebinding to the muxer socket!!");
                            std::mem::drop(listener);
                            loop {
                                listener = match TcpListener::bind(SocketAddrV4::new(
                                    Ipv4Addr::from_str("127.0.0.1").unwrap(),
                                    LISTEN_PORT,
                                )) {
                                    Ok(l) => l,
                                    Err(_) => {
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        continue;
                                    }
                                };
                                break;
                            }
                            info!("minimuxer has bound successfully");
                            retries = 0;

                            continue;
                        }
                    }
                };
                retries = 0;

                // Read the packet
                let mut buf = [0u8; 0xfff];
                let mut size = match stream.read(&mut buf) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Detect if only header was sent
                if size == 16 {
                    let mut buf2 = [0u8; 0xfff];
                    let new_size = match stream.read(&mut buf2) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let mut i = size;
                    for o in buf2 {
                        if i == buf.len() - 1 {
                            continue;
                        }
                        buf[i] = o;
                        i += 1;
                    }
                    size += new_size;
                }

                let packet: RawPacket = buf[..size].try_into().unwrap();

                // Handle the request
                let response = match handle_packet(&packet, pairing_file.clone()) {
                    Ok(res) => res,
                    Err(_) => continue,
                };

                let to_return: Vec<u8> = RawPacket::new(response, 1, 8, packet.tag).into();
                match stream.write_all(&to_return) {
                    Ok(_) => (),
                    Err(_) => continue,
                }
            }
        })
        .unwrap();
}

fn handle_packet(packet: &RawPacket, pairing_file: Plist) -> Result<Plist, PlistError> {
    match packet
        .plist
        .clone()
        .dict_get_item("MessageType")?
        .get_string_val()?
        .as_str()
    {
        "ListDevices" => {
            // Get the device UDID from the pairing file
            let udid = pairing_file.dict_get_item("UDID")?.get_string_val()?;

            // Create the return packet
            let mut to_return = Plist::new_array();

            let mut temp_dict = Plist::new_dict();
            temp_dict.dict_insert_item("DeviceID", Plist::new_uint(420))?;

            temp_dict.dict_insert_item("MessageType", "Attached".into())?;

            let mut properties_dict = Plist::new_dict();
            properties_dict.dict_insert_item("ConnectionType", "Network".into())?;
            properties_dict.dict_insert_item("DeviceID", Plist::new_uint(420))?;
            properties_dict.dict_insert_item("EscapedFullServiceName", "yurmomlolllllll".into())?;
            properties_dict.dict_insert_item("InterfaceIndex", Plist::new_uint(69))?;
            properties_dict.dict_insert_item(
                "NetworkAddress",
                convert_ip(IpAddr::V4(Ipv4Addr::from_str("10.7.0.1").unwrap()))
                    .to_vec()
                    .into(),
            )?;
            properties_dict.dict_insert_item("SerialNumber", udid.into())?;

            temp_dict.dict_insert_item("Properties", properties_dict)?;
            to_return.array_append_item(temp_dict)?;

            let mut upper = Plist::new_dict();
            upper.dict_set_item("DeviceList", to_return)?;

            Ok(upper)
        }
        "ReadPairRecord" => {
            let mut upper = Plist::new_dict();
            upper.dict_set_item(
                "PairRecordData",
                pairing_file.to_string().as_bytes().to_vec().into(),
            )?;

            Ok(upper)
        }
        _ => Err(PlistError::Unknown), // just a place-holder
    }
}

fn convert_ip(ip: IpAddr) -> [u8; 152] {
    let mut data = [0u8; 152];
    match ip {
        IpAddr::V4(ip_addr) => {
            data[0] = 10;
            data[1] = 0x02;
            data[2] = 0x00;
            data[3] = 0x00;
            let mut i = 4;
            for byte in ip_addr.octets() {
                data[i] = byte;
                i += 1;
            }
        }
        IpAddr::V6(ip_addr) => {
            data[0] = 28;
            data[1] = 0x1E;
            data[2] = 0x00;
            data[3] = 0x00;
            data[4] = 0x00;
            data[5] = 0x00;
            data[6] = 0x00;
            let mut i = 16;
            for byte in ip_addr.octets() {
                data[i] = byte;
                i += 1;
            }
        }
    }
    // Start from the back and fill with zeros
    let mut i = data.len() - 2;
    while i > 0 {
        if data[i] != 0 {
            break;
        }
        data[i] = 0;
        i -= 1;
    }

    data
}

#[cfg(not(test))]
pub static STARTED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
pub static STARTED: AtomicBool = AtomicBool::new(true); // minimuxer won't start in tests

/// Starts the muxer and heartbeat client
/// # Arguments
/// Pairing file contents as a string and log path as a string
pub fn start(pairing_file: String, log_path: String) -> crate::Result<()> {
    if STARTED.load(Ordering::Relaxed) {
        info!("Already started minimuxer, skipping");
        return Ok(());
    } else if std::fs::remove_file(&log_path).is_ok() { // only remove log file on first startup
    }

    // the logger failing to initialize isn't a problem since it will only fail if it has already been initialized
    if CombinedLogger::init(vec![
        TermLogger::new(
            // Allow debug logging for terminal only
            LevelFilter::max(),
            // Allow logging from everywhere, to include rusty_libimobiledevice and any other useful debugging info
            ConfigBuilder::new()
                .add_filter_ignore_str("plist_plus") // plist_plus spams logs
                .set_target_level(LevelFilter::Error)
                .build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new()
                .add_filter_allow("minimuxer".to_string())
                .build(),
            File::create(&log_path).unwrap(),
        ),
    ])
    .is_ok()
    {
        info!("Logger initialized!!");
    }

    let pairing_file = match Plist::from_xml(pairing_file) {
        Ok(p) => p,
        Err(_) => {
            error!("Failed to convert pairing file to plist!!");
            return Err(Errors::PairingFile);
        }
    };

    // TODO: compare this with fetch_udid() to ensure we have the correct pairing file, and in SideStore, tell the user if there's a mismatch
    // we can return Errors::UDIDMismatch
    let udid = match pairing_file.dict_get_item("UDID") {
        Ok(u) => match u.get_string_val() {
            Ok(s) => s,
            Err(e) => {
                error!("Couldn't convert UDID to string: {:?}", e);
                return Err(Errors::PairingFile);
            }
        },
        Err(e) => {
            error!("Couldn't get UDID: {:?}", e);
            return Err(Errors::PairingFile);
        }
    };

    listen(pairing_file);
    start_beat(udid);

    info!("minimuxer has started!");
    STARTED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Sets the current environment variable for libusbmuxd to localhost
pub fn target_minimuxer_address() {
    std::env::set_var("USBMUXD_SOCKET_ADDRESS", format!("127.0.0.1:{LISTEN_PORT}"));
}
