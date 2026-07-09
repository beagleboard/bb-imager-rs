//! This module contains a mock MSPM0 BSL simuator for testing.

use std::collections::VecDeque;

const CRC_ALGO: crc_fast::CrcAlgorithm = crc_fast::CrcAlgorithm::Crc32Jamcrc;

const CONNECTION_REQ: &[u8] = &[0x80, 0x01, 0, 0x12, 0x3A, 0x61, 0x44, 0xDE];
const DEVICE_INFO_REQ: &[u8] = &[0x80, 0x01, 0, 0x19, 0xB2, 0xB8, 0x96, 0x49];
const UNLOCK_REQ: &[u8] = &[
    0x80, 0x21, 0, 0x21, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0xAA, 0xF0, 0x3D,
];
const MASS_ERASE_REQ: &[u8] = &[0x80, 0x01, 0, 0x15, 0x99, 0xF4, 0x20, 0x40];
const START_APPLICATION_REQ: &[u8] = &[0x80, 0x01, 0x00, 0x40, 0xE2, 0x51, 0x21, 0x5B];
const CHANGE_BAUD_RATE_REQ: &[u8] = &[0x80, 0x02, 0, 0x52, 0x03, 0x6C, 0x83, 0xA2, 0xAF];

#[derive(Debug, Clone, Copy)]
enum State {
    Waiting,
    Connected,
    Unlocked,
    WaitingForData((usize, crc_fast::Digest)),
    WaitingForCrc(crc_fast::Digest),
}

pub struct MockBsl {
    state: State,
    tx_data: VecDeque<u8>,
    is_uart: bool,
    flash: Vec<u8>,
}

impl MockBsl {
    pub const fn uart() -> Self {
        Self {
            state: State::Waiting,
            tx_data: VecDeque::new(),
            is_uart: true,
            flash: Vec::new(),
        }
    }
}

impl Default for MockBsl {
    fn default() -> Self {
        Self {
            state: State::Waiting,
            tx_data: VecDeque::new(),
            is_uart: false,
            flash: Vec::new(),
        }
    }
}

impl std::io::Write for MockBsl {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match (buf, self.state) {
            (CONNECTION_REQ, State::Waiting) => {
                self.tx_data.push_back(0);
                self.state = State::Connected;
            }
            (DEVICE_INFO_REQ, State::Connected) => {
                self.tx_data
                    .write(&[
                        0, 0x08, 0x19, 0, 0x31, 0, 0x01, 0, 0x01, 0, 0, 0, 0, 0x01, 0, 0xC0, 0x06,
                        0x60, 0x01, 0, 0x20, 0x01, 0, 0, 0, 0x01, 0, 0, 0, 0x49, 0x61, 0x57, 0x8C,
                    ])
                    .unwrap();
            }
            (UNLOCK_REQ, State::Connected) => {
                self.state = State::Unlocked;
                self.tx_data
                    .write(&[0, 0x08, 0x02, 0, 0x3B, 0, 0x38, 0x02, 0x94, 0x82])
                    .unwrap();
            }
            (MASS_ERASE_REQ, State::Unlocked) => {
                self.tx_data
                    .write(&[0, 0x08, 0x02, 0, 0x3B, 0, 0x38, 0x02, 0x94, 0x82])
                    .unwrap();
            }
            (START_APPLICATION_REQ, State::Connected)
            | (START_APPLICATION_REQ, State::Unlocked) => {
                self.tx_data.push_back(0);
            }
            (CHANGE_BAUD_RATE_REQ, State::Connected) | (CHANGE_BAUD_RATE_REQ, State::Unlocked)
                if self.is_uart =>
            {
                self.tx_data.push_back(0)
            }
            ([0x80, 0x09, 0, 0x26, ..], State::Unlocked) => {
                let flash_crc = {
                    let mut crc = crc_fast::Digest::new(CRC_ALGO);
                    crc.update(&self.flash);
                    u32::try_from(crc.finalize()).unwrap()
                };

                self.tx_data.write_all(&[0, 0x08, 0x05, 0, 0x32]).unwrap();
                self.tx_data.write_all(&flash_crc.to_le_bytes()).unwrap();

                let msg_crc = {
                    let mut crc = crc_fast::Digest::new(CRC_ALGO);
                    crc.update(&[0x32]);
                    crc.update(&flash_crc.to_le_bytes());
                    u32::try_from(crc.finalize()).unwrap()
                };

                self.tx_data.write_all(&msg_crc.to_le_bytes()).unwrap();
            }
            ([0x80, _, _, 0x20, ..], State::Unlocked) => {
                let core_data_len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
                let data_len = core_data_len - size_of::<u8>() - size_of::<u32>();
                assert_eq!(data_len % 8, 0);

                let addr = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
                assert_eq!(addr % 8, 0);

                self.flash.resize(addr as usize, 0xff);

                let mut crc = crc_fast::Digest::new(CRC_ALGO);

                crc.update(&buf[3..]);

                self.state = State::WaitingForData((data_len, crc));
            }
            (_, State::WaitingForData((x, mut crc))) => {
                self.flash.extend(buf);
                crc.update(buf);
                let left = x - buf.len();
                if left == 0 {
                    self.state = State::WaitingForCrc(crc)
                } else {
                    self.state = State::WaitingForData((left, crc))
                }
            }
            (_, State::WaitingForCrc(crc)) => {
                let expected_crc = u32::try_from(crc.finalize()).unwrap();
                assert_eq!(buf.len(), 4);
                let got_crc = u32::from_le_bytes(buf.try_into().unwrap());

                assert_eq!(got_crc, expected_crc);

                self.state = State::Unlocked;
                self.tx_data
                    .write(&[0, 0x08, 0x02, 0, 0x3B, 0, 0x38, 0x02, 0x94, 0x82])
                    .unwrap();
            }
            _ => panic!("Unexpected Request: {:x?}", buf),
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for MockBsl {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.tx_data.read(buf)
    }
}
