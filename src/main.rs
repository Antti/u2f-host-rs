extern crate u2f_hid;

use u2f_hid::Manager;
use u2f_hid::APDU;

fn main() {
    let nonce = [0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1];

    let manager = Manager::new().expect("Cant init manager");
    for mut dev in manager.discover() {
        let init_result = dev.init(nonce);
        println!("Init result: {:?}", init_result);
        dev.wink().expect("Cant wink");
        // dev.ping([1,2,3,4]);
    }
    for mut dev in manager.discover() {
        let init_result = dev.init(nonce);
        println!("Init result: {:?}", init_result);
        let mut apdu = APDU::new(dev);
        apdu.send_apdu(1, 1, vec![0]).expect("Cant send APDU");
        // dev.ping([1,2,3,4]);
    }
}
