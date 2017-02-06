use hid;
use super::errors::*;
use super::HidCmd;
use super::u2f_hid_framed_transport::U2FHidFramedTransport;
use std::marker::PhantomData;
use std::time::Duration;
use std::convert::From;
use std::fmt;

const FIDO_USAGE_PAGE : u16 = 0xf1d0;
const FIDO_USAGE_U2FHID : u16 = 0x01;

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

impl <'a> Manager<'a> {
    pub fn new() -> Result<Self> {
        let hid_manager = hid::init()?;
        Ok(Manager { hid_manager: hid_manager, _marker: PhantomData })
    }

    pub fn discover(&'a self) -> Devices<'a> {
        Devices { hid_devices: self.hid_manager.devices() }
    }
}

impl <'a> fmt::Debug for Device<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Device")
            .field("channel_id", &self.channel_id)
            .field("hid_device_info", &self.hid_device_info)
            .finish()
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
        let channel_id = self.channel_id;
        self.send_cmd(cmd, channel_id, data)?;
        self.read_response(cmd)
    }
}

impl <'a> U2FHidFramedTransport for Device<'a> {
    fn data_read<T: AsMut<[u8]>>(&mut self, buffer: T, timeout: Duration) -> Result<Option<usize>> {
        self.handle.data().read(buffer, timeout).map_err(|e| e.into())
    }
    fn data_write<T: AsRef<[u8]>>(&mut self, buffer: T) -> Result<usize> {
        self.handle.data().write(buffer).map_err(|e| e.into())
    }
}
