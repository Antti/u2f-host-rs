extern crate hid;
extern crate byteorder;
mod device;
mod u2f_frame;

pub use device::Manager;

#[cfg(test)]
mod tests {
    use super::device::Manager;
    #[test]
    fn it_works() {
        let manager = Manager::new();
        for mut dev in manager.discover() {
            let init_result = dev.init();
            println!("Init result: {:?}", init_result);
            dev.wink();
            // dev.ping([1,2,3,4]);
        }
        assert!(false);
    }
}
