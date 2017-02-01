/// Frame Content can be one of two types, ither the inial frame or continuation
pub enum U2FFrameContent {
    Init { cmd: u8, data_len: u16, data: Vec<u8> }, // data len 57:  64-7.
    Cont { seq: u8, data: Vec<u8> } // data len: 59 64-5
}

pub struct U2FFrame {
    pub channel_id: u32,
    pub frame_content: U2FFrameContent
}

impl U2FFrame {
    pub fn as_bytes(&self) -> Vec<u8> {
        use std::io::Write;
        use byteorder::{BigEndian, WriteBytesExt};

        let mut bytes = vec![];
        bytes.write_u32::<BigEndian>(self.channel_id).unwrap();

        match self.frame_content {
            U2FFrameContent::Init { cmd, data_len, ref data } => {
                bytes.write_u8(cmd | 0x80).unwrap();
                bytes.write_u16::<BigEndian>(data_len).unwrap();
                bytes.write_all(&data).unwrap();
            },
            U2FFrameContent::Cont { seq, ref data } => {
                bytes.write_u8(seq & !0x80u8).unwrap();
                bytes.write_all(&data).unwrap();
            }
        }
        bytes.resize(64, 0);
        bytes
    }
}
