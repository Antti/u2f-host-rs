use hid;
use super::u2f_frame::*;
use super::errors::*;
use super::HidCmd;
use std::marker::PhantomData;
use std::cmp;
use std::time::Duration;
use std::convert::From;

#[derive(Debug)]
pub struct HIDDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer_string: String,
    pub product_string: String
}

#[derive(Debug)]
pub struct U2FDeviceInfo {
    pub protocol_version: u8,
    pub version_major: u8,
    pub version_minor: u8,
    pub version_build: u8,
    pub cap_flags: u8
}

// TODO: Store hid::Device here, not just handle.
// Split Device into 2 parts: FoundDevice (not inited, channel 0xffffffff) & Device(inited)
pub struct Device<'a> {
    handle: hid::Handle,
    channel_id: u32,
    pub hid_device_info: HIDDeviceInfo,

    _marker: PhantomData<&'a ()>,
}

pub struct Manager<'a> {
    hid_manager: hid::Manager,

    _marker: PhantomData<&'a ()>
}

pub struct Devices<'a> {
    hid_devices: hid::Devices<'a>,
}

impl<'a> Iterator for Devices<'a> {
	type Item = Device<'a>;

	fn next(&mut self) -> Option<Self::Item> {
        loop {
            let hid_device = self.hid_devices.next();
            match hid_device {
                Some(hid_device) => {
                    if hid_device.usage_page() == FIDO_USAGE_PAGE && hid_device.usage() == FIDO_USAGE_U2FHID {
                        let dev = Device {
                            handle: hid_device.open().unwrap(),
                            hid_device_info: HIDDeviceInfo {
                                vendor_id: hid_device.vendor_id(),
                                product_id: hid_device.product_id(),
                                manufacturer_string: hid_device.manufacturer_string().unwrap(),
                                product_string: hid_device.product_string().unwrap()
                            },
                            channel_id: 0xffffffff, // Broadcast
                            _marker: PhantomData
                        };
                        println!("Discovered U2F Device: {:?}", dev.hid_device_info);
                        return Some(dev);
                    }
                },
                None => return None
            }
        }
	}
}

const FIDO_USAGE_PAGE : u16 = 0xf1d0;
const FIDO_USAGE_U2FHID : u16 = 0x01;

impl <'a> Manager<'a> {
    pub fn new() -> Result<Self> {
        let hid_manager = hid::init()?;
        Ok(Manager { hid_manager: hid_manager, _marker: PhantomData })
    }

    pub fn discover(&'a self) -> Devices<'a> {
        Devices { hid_devices: self.hid_manager.devices() }
    }
}

impl <'a> Device<'a> {
    pub fn init(&mut self) -> Result<U2FDeviceInfo> {
        use std::io::{Read, Cursor};
        use byteorder::{BigEndian, ReadBytesExt};

        let nonce = &[0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1];
        let data = self.request(HidCmd::Init as u8, nonce)?;
        println!("Received init frame response: {:?}", data);

        let mut rdr = Cursor::new(data);
        let mut nonce_response = Vec::with_capacity(8);
        rdr.read_exact(&mut nonce_response)?;
        // println!("Nonce response: {:?}", nonce_response);
        // TODO: Make sure nonce response_matches nonce.
        self.channel_id = rdr.read_u32::<BigEndian>()?;
        let protocol_version = rdr.read_u8()?;
        let version_major = rdr.read_u8()?;
        let version_minor = rdr.read_u8()?;
        let version_build = rdr.read_u8()?;
        let cap_flags = rdr.read_u8()?;
        Ok(U2FDeviceInfo {
            protocol_version: protocol_version,
            version_major: version_major,
            version_minor: version_minor,
            version_build: version_build,
            cap_flags: cap_flags
        })
    }

    // TODO: Check capabilities first
    pub fn wink(&mut self) -> Result<()> {
        self.request(HidCmd::Wink as u8, []).map(|_| ()).map_err(From::from)
    }

    pub fn ping<T: AsRef<[u8]>>(&mut self, data: T) -> Result<Vec<u8>> {
        let data = data.as_ref();
        Ok(self.request(HidCmd::Ping as u8, data)?)
    }

    /// High level U2F device api to perform HID command and read response.
    /// The data is correctly framed (in 64kb frames) and sent over the HID interface.
    /// The response is read from one or more frames, validated and the data is returned back
    pub fn request<T: AsRef<[u8]>>(&mut self, cmd: u8, data: T) -> Result<Vec<u8>> {
        self.send_cmd(cmd, data)?;
        self.read_response(cmd)
    }

    /// High level send cmd with data api
    fn send_cmd<T: AsRef<[u8]>>(&mut self, cmd: u8, data: T) -> Result<()> {
        let data = data.as_ref();
        let mut datasent = 0;
        let mut sequence = 0;

        let frame_data = &data[datasent .. cmp::min(data.len(), 57)];
        let frame = U2FFrame {
           channel_id: self.channel_id,
           frame_content: U2FFrameContent::Init { cmd: cmd, data_len: data.len() as u16, data: frame_data.to_vec() }
        };

        self.send_frame(&frame)?;
        datasent += frame_data.len();

        while data.len() > datasent {
            let frame_data = &data[datasent .. cmp::min(data.len() - datasent, 59)];
            let frame = U2FFrame {
               channel_id: self.channel_id,
               frame_content: U2FFrameContent::Cont { seq: sequence, data: frame_data.to_vec() }
            };
            self.send_frame(&frame)?;

            sequence += 1;
            datasent += frame_data.len();
        }
        Ok(())
    }

    /// High level read response
    fn read_response(&mut self, expected_cmd: u8) -> Result<Vec<u8>> {
        let init_frame = self.read_frame()?;
        let (cmd, total_data_len, mut data) = match init_frame.frame_content {
            U2FFrameContent::Init { cmd, data_len, data } => (cmd, data_len, data),
            _ => return Err(ErrorKind::Protocol("Expected init frame".to_string()).into())
        };
        if cmd != expected_cmd {
            return Err(
                ErrorKind::Protocol(
                    format!("Unexpected response cmd. Expected 0x{:x}. Got 0x{:x}", expected_cmd, cmd)
                ).into()
            )
        }
        let mut current_sequence = 0;
        let curr_data_len = cmp::min(data.len() as u16, total_data_len);
        data.reserve((total_data_len - curr_data_len) as usize);
        while data.len() < total_data_len as usize {
            let cont_frame = self.read_frame()?;
            let (seq, mut cont_data) = match cont_frame.frame_content {
                U2FFrameContent::Cont { seq, data } => (seq, data),
                _ => return Err(
                    ErrorKind::Protocol("Expected cont frame. Got init".to_string()).into()
                )
            };
            if seq == current_sequence {
                data.append(&mut cont_data);
            } else {
                return Err(
                    ErrorKind::Protocol(
                        format!("Sequence error. Expected: {}. Got: {}", current_sequence, seq)
                    ).into()
                )
            }
            current_sequence += 1;
        }
        data.resize(total_data_len as usize, 0); // Strip remaining 0s if any
        Ok(data)
    }

    fn read_frame(&mut self) -> Result<U2FFrame> {
        let mut buffer = vec![0u8; 64];
        let res = self.handle.data().read(&mut buffer, Duration::from_millis(500))?;
        match res {
            Some(data_len) if data_len == 64 => {}
            Some(other_len) => return Err(
                ErrorKind::Protocol(
                    format!("HID read returned unexpected data size: {}. Expected 64", other_len)
                ).into()
            ),
            None => return Err(ErrorKind::Protocol("Nothing was returned from device".to_string()).into())
        }
        let frame = U2FFrame::from_bytes(buffer)?;
        println!("Received frame: {:?}", frame);
        Ok(frame)
    }

    fn send_frame(&mut self, frame: &U2FFrame) -> Result<()> {
        let mut frame_bytes = frame.as_bytes();
        frame_bytes.insert(0, 0); // TODO: Check if report 0 correct
        println!("Sending frame: {:?}", frame_bytes);
        self.handle.data().write(frame_bytes).map(|_| ()).map_err(From::from)
    }
}
