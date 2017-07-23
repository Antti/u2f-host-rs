use super::device::Device;
use super::errors::*;
use super::HidCmd;

#[derive(Debug)]
pub struct APDU<'a>(Device<'a>);
pub type APDUResponse = Vec<u8>;

impl<'a> APDU<'a> {
    pub fn new(dev: Device<'a>) -> Self {
        APDU(dev)
    }

    // the request-data are preceded by three length bytes, a byte with value 0 followed by the length of request-data, in big-endian order.
    pub fn send_apdu<T: AsRef<[u8]>>(&mut self, cmd: u8, p1: u8, p2: u8, cmd_data: T) -> Result<APDUResponse> {
        use std::io::Write;
        use byteorder::{BigEndian, WriteBytesExt};

        let cmd_data = cmd_data.as_ref();

        let mut data = Vec::with_capacity(cmd_data.len() + 7 + 2);
        data.write_u8(0)?; // CLA
        data.write_u8(cmd)?; // INS
        data.write_u8(p1)?; // p1
        data.write_u8(p2)?; // p2

        data.write_u8(0)?; // lengths are preceeded with 0
        if cmd_data.len() > 0 {
            data.write_u16::<BigEndian>(cmd_data.len() as u16)?; // data len
            data.write_all(cmd_data)?;
        }
        // When Ne = 65 536, let Le1 = 0 and Le2 = 0., so when we don't have limit, then set bytes to 0
        data.write_u16::<BigEndian>(0u16)?; // part of response data len

        let mut response = self.0.request(HidCmd::Msg as u8, data)?;
        let response_len = response.len();
        let sw = (response[response_len - 2] as u16) << 8 | (response[response_len - 1] as u16);
        response.resize(response_len - 2, 0);
        match sw {
            0x9000 => Ok(response), // SW_NO_ERROR
            0x6984 => Err(ErrorKind::APDUError(sw, "wrong data", response).into()), // SW_WRONG_DATA
            0x6985 => Err(ErrorKind::APDUError(sw, "conditions not satisfied", response).into()), // SW_CONDITIONS_NOT_SATISFIED
            0x6d00 => Err(ErrorKind::APDUError(sw, "ins not supported", response).into()), // SW_INS_NOT_SUPPORTED
            0x6e00 => Err(ErrorKind::APDUError(sw, "cla not supported", response).into()), // SW_CLA_NOT_SUPPORTED
            _ => Err(ErrorKind::Protocol(format!("Unknown APDU status code: {}", sw)).into()),
        }
    }
}
