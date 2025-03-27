// Below is a version of the `main` function and some error types. This assumes
// the existence of types like `FileManager`, `Packet`, and `PacketParseError`.
// You can use this code as a starting point for the exercise, or you can
// delete it and write your own code with the same function signature.


use std::{
    io::{self, Write},
    net::UdpSocket,
    ffi:: OsString, // Storing OS-compatible filenames
    convert::TryFrom, // Implement TryFrom trait for Packet
    collections::HashMap, // HashMap for storing file packets
};

enum Packet {
    // Define the packet structure here
    Header (Header), // header packet with file name
    Data (Data), // data packet with file content
}

#[derive(Debug, PartialEq, Eq)]
struct Header {
    file_id: u8,
    file_name: OsString,
}

#[derive(Debug, PartialEq, Eq)]
struct Data {
    file_id: u8,
    packet_number: u16,
    is_last_packet: bool,
    data: Vec<u8>, // file content
}

#[derive(Debug)]
pub  struct PacketParseError {
    message: String,
}

impl TryFrom<&[u8]> for Packet {
    type Error = PacketParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 2 {
            return Err(PacketParseError {
                message: "Packet too short".to_string(),
            });
        }

        let status = bytes[0]; // First byte is status byte
        let file_id = bytes[1]; // Second byte is file ID

        if status % 2 == 0 {
            // Header packet case
            let file_name = String::from_utf8(bytes[2..].to_vec())
                .map_err(|_| PacketParseError {
                    message: "Invalid UTF-8 sequence".to_string()})?;

            Ok(Packet::Header(Header {
                file_id,
                file_name: OsString::from(file_name),
            }))
            } else {
                // Data packet case
                if bytes.len() < 4 {
                    return Err(PacketParseError {
                        message: "Data packet too short".to_string(),
                    });
                }

                let packet_number = u16::from_be_bytes([bytes[2], bytes[3]]); // Parse 2 byte big endian packet num
                let is_last_packet = status % 4 == 3; // check last packet if status % 4 = = 3 
                let data = bytes[4..].to_vec(); // data content
                Ok(Packet::Data(Data {
                    file_id,
                    packet_number,
                    is_last_packet,
                    data,
                }))
            }
        }
    }

// Manage and store files into disk
#[derive(Default)]
struct FileManager {
    files: HashMap<u8, (Option<OsString>, Option<u16>, HashMap<u16, Vec<u8>>)>, // Mpas file ID to PacketGroup
}








#[derive(Debug)]
pub enum ClientError {
    IoError(std::io::Error),
    PacketParseError(PacketParseError),
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::IoError(e)
    }
}

impl From<PacketParseError> for ClientError {
    fn from(e: PacketParseError) -> Self {
        Self::PacketParseError(e)
    }
}

fn main() -> Result<(), ClientError> {
    let sock = UdpSocket::bind("0.0.0.0:7077")?;

    let remote_addr = "127.0.0.1:6014";
    sock.connect(remote_addr)?;
    let mut buf = [0; 1028];

    let _ = sock.send(&buf[..1028]);

    let mut file_manager = FileManager::default();

    while !file_manager.received_all_packets() {
        let len = sock.recv(&mut buf)?;
        let packet: Packet = buf[..len].try_into()?;
        print!(".");
        io::stdout().flush()?;
        file_manager.process_packet(packet);
    }

    file_manager.write_all_files()?;

    Ok(())
}


