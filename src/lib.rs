#![recursion_limit = "1024"]

extern crate hid;
extern crate byteorder;
#[macro_use]
extern crate error_chain;

mod device;
mod u2f_frame;
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
        }
    }
}

pub use device::Manager;
pub use errors::*;

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
