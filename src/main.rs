extern crate u2f_hid;
extern crate crypto;

use u2f_hid::{APDU, Manager, U2FINS};
use crypto::digest::Digest;
use crypto::sha2::Sha256;

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
        let mut hasher = Sha256::new();
        hasher.input_str("hello world");
        let mut buf1 = [0; 32];
        let mut buf2 = [0; 32];
        hasher.result(&mut buf1);
        hasher.result(&mut buf2);
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&buf1);
        buf.extend_from_slice(&buf2);
        let response = apdu.send_apdu(U2FINS::Register as u8, 0, 0, &buf);
        println!("APDU Response: {:?}", response);

    }
}
