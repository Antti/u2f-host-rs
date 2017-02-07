use super::u2f_frame::*;
use super::errors::*;
use std::cmp;
use std::time::Duration;

pub trait U2FHidFramedTransport {
    fn data_read<T: AsMut<[u8]>>(&mut self, buffer: T, timeout: Duration) -> Result<Option<usize>>;
    fn data_write<T: AsRef<[u8]>>(&mut self, buffer: T) -> Result<usize>;
    /// High level read response
    fn read_response(&mut self, expected_cmd: u8) -> Result<Vec<u8>> {
        let init_frame = self.read_frame()?;
        let (cmd, total_data_len, mut data) = match init_frame.frame_content {
            U2FFrameContent::Init { cmd, data_len, data } => (cmd, data_len, data),
            _ => return Err(ErrorKind::Protocol("Expected init frame".to_string()).into())
        };
        if cmd != expected_cmd {
            return Err(
                ErrorKind::Protocol(
                    format!("Unexpected response cmd. Expected 0x{:x}. Got 0x{:x}", expected_cmd, cmd)
                ).into()
            )
        }
        let mut current_sequence = 0;
        let curr_data_len = cmp::min(data.len() as u16, total_data_len);
        data.reserve((total_data_len - curr_data_len) as usize);
        while data.len() < total_data_len as usize {
            let cont_frame = self.read_frame()?;
            let (seq, mut cont_data) = match cont_frame.frame_content {
                U2FFrameContent::Cont { seq, data } => (seq, data),
                _ => return Err(
                    ErrorKind::Protocol("Expected cont frame. Got init".to_string()).into()
                )
            };
            if seq == current_sequence {
                data.append(&mut cont_data);
            } else {
                return Err(
                    ErrorKind::Protocol(
                        format!("Sequence error. Expected: {}. Got: {}", current_sequence, seq)
                    ).into()
                )
            }
            current_sequence += 1;
        }
        data.resize(total_data_len as usize, 0); // Strip remaining 0s if any
        Ok(data)
    }

    /// High level send cmd with data api
    fn send_cmd<T: AsRef<[u8]>>(&mut self, cmd: u8, channel_id: u32, data: T) -> Result<()> {
        let data = data.as_ref();
        let mut datasent = 0;
        let mut sequence = 0;

        let frame_data = &data[datasent .. cmp::min(data.len(), 57)];
        let frame = U2FFrame {
           channel_id: channel_id,
           frame_content: U2FFrameContent::Init { cmd: cmd, data_len: data.len() as u16, data: frame_data.to_vec() }
        };

        self.send_frame(&frame)?;
        datasent += frame_data.len();

        while data.len() > datasent {
            let frame_data = &data[datasent .. cmp::min(data.len() - datasent, 59)];
            let frame = U2FFrame {
               channel_id: channel_id,
               frame_content: U2FFrameContent::Cont { seq: sequence, data: frame_data.to_vec() }
            };
            self.send_frame(&frame)?;

            sequence += 1;
            datasent += frame_data.len();
        }
        Ok(())
    }

    fn read_frame(&mut self) -> Result<U2FFrame> {
        let mut buffer = vec![0u8; 64];
        let res = self.data_read(&mut buffer, Duration::from_millis(500))?;
        match res {
            Some(data_len) if data_len == 64 => {}
            Some(other_len) => return Err(
                ErrorKind::Protocol(
                    format!("HID read returned unexpected data size: {}. Expected 64", other_len)
                ).into()
            ),
            None => return Err(ErrorKind::Protocol("Nothing was returned from device".to_string()).into())
        }
        let frame = U2FFrame::from_bytes(buffer)?;
        println!("Received frame: {:?}", frame);
        Ok(frame)
    }

    fn send_frame(&mut self, frame: &U2FFrame) -> Result<()> {
        println!("Sending frame: {:?}", frame);
        let mut frame_bytes = frame.as_bytes()?;
        // println!("Frame bytes: {:?}", frame_bytes);
        frame_bytes.insert(0, 0); // TODO: Check if report 0 correct
        self.data_write(frame_bytes).map(|_| ()).map_err(|e| e.into())
    }
}
