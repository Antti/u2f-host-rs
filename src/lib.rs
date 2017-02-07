#![recursion_limit = "1024"]

extern crate hid;
extern crate byteorder;
#[macro_use] extern crate error_chain;

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
            APDUError(error_code: u16, desc: &'static str) {
                description(desc)
                display("error code: {}, desc: {}", error_code, desc)
            }
        }
    }
}

pub use device::{Device, Devices, Manager, HIDDeviceInfo, U2FDeviceInfo};
pub use errors::*;
pub use apdu::APDU;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum HidCmd {
    Ping = 0x01,
    Msg = 0x03,
    Lock = 0x04,
    Init = 0x06,
    Wink = 0x08,
    Error = 0x3f
}

#[cfg(test)]
mod tests {
    use super::device::Manager;
    #[test]
    fn it_works() {
        let manager = Manager::new().unwrap();
        for mut dev in manager.discover() {
            let init_result = dev.init();
            println!("Init result: {:?}", init_result);
            dev.wink().unwrap();
            // dev.ping([1,2,3,4]);
        }
        assert!(false);
    }
}
