use super::errors::*;

/// Frame Content can be one of two types, ither the inial frame or continuation
#[derive(Debug)]
pub enum U2FFrameContent {
    Init { cmd: u8, data_len: u16, data: Vec<u8> }, // data len 57:  64-7.
    Cont { seq: u8, data: Vec<u8> } // data len: 59 64-5
}

#[derive(Debug)]
pub struct U2FFrame {
    pub channel_id: u32,
    pub frame_content: U2FFrameContent
}

impl U2FFrame {
    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        use std::io::Write;
        use byteorder::{BigEndian, WriteBytesExt};

        let mut bytes = vec![];
        bytes.write_u32::<BigEndian>(self.channel_id)?;

        match self.frame_content {
            U2FFrameContent::Init { cmd, data_len, ref data } => {
                bytes.write_u8(cmd | 0x80)?;
                bytes.write_u16::<BigEndian>(data_len)?;
                bytes.write_all(&data)?;
            },
            U2FFrameContent::Cont { seq, ref data } => {
                bytes.write_u8(seq & !0x80u8)?;
                bytes.write_all(&data)?;
            }
        }
        bytes.resize(64, 0);
        Ok(bytes)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(data: T) -> Result<Self> {
        use std::io::{Read, Cursor};
        use byteorder::{BigEndian, ReadBytesExt};

        let mut rdr = Cursor::new(data.as_ref());
        let channel_id = rdr.read_u32::<BigEndian>()?;
        let cmd = rdr.read_u8()?;
        let frame_content = match cmd & 0x80 {
            0 => { // Continuation
                let mut data = Vec::with_capacity(59);
                rdr.read_to_end(&mut data)?;
                U2FFrameContent::Cont { seq: cmd, data: data }
            },
            0x80 => { // Init
                let data_len = rdr.read_u16::<BigEndian>()?;
                let mut data = Vec::with_capacity(57);
                rdr.read_to_end(&mut data)?;
                U2FFrameContent::Init { cmd: cmd & !0x80u8, data_len: data_len, data: data }
            },
            _ => unreachable!()
        };
        Ok(U2FFrame { channel_id: channel_id, frame_content: frame_content })
    }
}
