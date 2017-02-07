extern crate u2f_hid;

use u2f_hid::{APDU, Manager, U2FINS};

fn main() {
    let nonce = [0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1];

    let manager = Manager::new().expect("Cant init manager");
    for mut dev in manager.discover() {
        let init_result = dev.init(nonce);
        println!("Init result: {:?}", init_result);
        dev.wink().expect("Cant wink");
        let mut apdu = APDU::new(dev);
        let response = apdu.send_apdu(U2FINS::Version as u8, 0, 0, vec![]).expect("Cant send APDU"); //
        println!("APDU Response: {:?}", response);
        println!("APDU Response: {:?}", String::from_utf8_lossy(&response));
    }
}
