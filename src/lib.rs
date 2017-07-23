#![recursion_limit = "1024"]

extern crate hid;
extern crate byteorder;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
// #[macro_use] extern crate enum_primitive;

mod device;
mod u2f_frame;
mod apdu;
mod u2f_hid_framed_transport;

mod errors {
    use hid;
    use std;
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {
        foreign_links {
            Hid(hid::Error);
            IO(std::io::Error);
        }

        errors {
            Protocol(t: String) {
                description("protocol error")
                display("protocol error: '{}'", t)
            }
            APDUError(error_code: u16, desc: &'static str, data: Vec<u8>) {
                description(desc)
                display("error code: {}, desc: {}", error_code, desc)
            }
        }
    }
}

pub use device::{Device, Devices, Manager, HIDDeviceInfo, U2FDeviceInfo};
pub use errors::*;
pub use apdu::APDU;

// enum_from_primitive! {
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum HidCmd {
    Ping = 0x01,
    Msg = 0x03,
    Lock = 0x04,
    Init = 0x06,
    Wink = 0x08,
    Error = 0x3f,
}
// }

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum U2FINS {
    Register = 0x01,
    Authenticate = 0x02,
    Version = 0x03,
    VendorFirst = 0x40,
    VendorLast = 0x7f,
}

#[cfg(test)]
mod tests {
    use super::device::Manager;
    use super::apdu::APDU;
    #[test]
    fn test_wink() {
        let manager = Manager::new().unwrap();
        for mut dev in manager.discover() {
            let init_result = dev.init([0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1]);
            println!("Init result: {:?}", init_result);
            dev.wink().unwrap();
            // dev.ping([1,2,3,4]);
        }
    }

    #[test]
    fn test_apdu() {
        return;
        let manager = Manager::new().unwrap();
        for mut dev in manager.discover() {
            let init_result = dev.init([0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1]);
            println!("Init result: {:?}", init_result);
            let mut apdu = APDU::new(dev);
            // apdu.send_apdu(1, 1, vec![0]).unwrap();
            // dev.ping([1,2,3,4]);
        }
    }
}
