use zerocopy::{FromBytes, Immutable, IntoBytes, little_endian};

use crate::{Error, Result};

const CRC_ALGO: crc_fast::CrcAlgorithm = crc_fast::CrcAlgorithm::Crc32Jamcrc;

// Bootloader Core Commands
const COMMAND_CONNECTION: u8 = 0x12;
const COMMAND_UNLOCK_BOOTLOADER: u8 = 0x21;
const COMMAND_MASS_ERASE: u8 = 0x15;
const COMMAND_PROGRAM_DATA: u8 = 0x20;
const COMMAND_GET_DEVICE_INFO: u8 = 0x19;
const COMMAND_STANDALONE_VERIFICATION: u8 = 0x26;
const COMMAND_START_APPLICATION: u8 = 0x40;

// BSL Acknowledgment
const BSL_ACK: u8 = 0x00;
const BSL_ERROR_HEADER_INCORRECT: u8 = 0x51;
const BSL_ERROR_CHECKSUM_INCORRECT: u8 = 0x52;
const BSL_ERROR_PACKET_SIZE_ZERO: u8 = 0x53;
const BSL_ERROR_PACKET_SIZE_TOO_BIG: u8 = 0x54;
const BSL_ERROR_UNKNOWN_BAUD_RATE: u8 = 0x56;

// BSL packet headers
const REQUEST: u8 = 0x80;
const RESPONSE: u8 = 0x08;

// BSL response byte
const GET_DEVICE_INFO: u8 = 0x31;
const CORE_MESSAGE: u8 = 0x3b;
const STANDALONE_VERIFICATION: u8 = 0x32;
const DEFAULT_BSL_MAX_BUFFER_SIZE: usize = 0x06c0;

// BSL core message response msg byte
const OPERATION_SUCCESSFUL: u8 = 0x00;

pub(crate) fn crc(buf: &[u8]) -> little_endian::U32 {
    u32::try_from(crc_fast::checksum(CRC_ALGO, buf))
        .unwrap()
        .into()
}

#[derive(FromBytes, IntoBytes, Immutable, Debug, Clone, Copy)]
#[repr(C, packed)]
struct BSLPktHead {
    header: u8,
    len: little_endian::U16,
    cmd: u8,
}

impl BSLPktHead {
    fn new_req(len: u16, cmd: u8) -> Self {
        Self {
            header: REQUEST,
            len: len.into(),
            cmd,
        }
    }

    fn validate_resp(&self, cmd: u8, pkt_len: u16) -> Result<()> {
        if self.header != RESPONSE {
            tracing::error!("Expected response");
            return Err(Error::InvalidResponse);
        }

        if self.cmd != cmd {
            tracing::error!("Unexpected response cmd: {}", self.cmd);
            return Err(Error::InvalidResponse);
        }

        if self.len != pkt_len {
            tracing::error!("Unexpected response length: {}", self.len);
            return Err(Error::InvalidResponse);
        }

        Ok(())
    }
}

type BSLPktCrc32 = little_endian::U32;

#[derive(IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLNoDataReqPkt {
    head: BSLPktHead,
    tail: BSLPktCrc32,
}

impl BSLMsg for BSLNoDataReqPkt {}

impl BSLNoDataReqPkt {
    fn new(cmd: u8) -> Self {
        Self {
            head: BSLPktHead::new_req(Self::len(), cmd),
            tail: crc(&[cmd]),
        }
    }

    fn connection_req() -> Self {
        Self::new(COMMAND_CONNECTION)
    }

    fn get_device_info_req() -> Self {
        Self::new(COMMAND_GET_DEVICE_INFO)
    }
}

#[derive(FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C, packed)]
struct BSLDeviceInfoRespPkt {
    head: BSLPktHead,
    cmd_interpreter_version: little_endian::U16,
    build_id: little_endian::U16,
    application_version: little_endian::U32,
    active_plugin_version: little_endian::U16,
    bsl_max_buffer_size: little_endian::U16,
    bsl_buffer_start_address: little_endian::U32,
    bcr_configuration_id: little_endian::U32,
    bsl_configuration_id: little_endian::U32,
    tail: BSLPktCrc32,
}

trait Crc32 {
    fn crc32(&self) -> little_endian::U32;
}

// Empty trait to show that it is a MSPM0 BSL message
trait BSLMsg: Sized {
    fn len() -> u16 {
        (size_of::<Self>() - size_of::<BSLNoDataReqPkt>() + size_of::<u8>()) as u16
    }
}

impl<T> Crc32 for T
where
    T: IntoBytes + Immutable + BSLMsg,
{
    fn crc32(&self) -> little_endian::U32 {
        crc(&self.as_bytes()[(size_of::<BSLPktHead>() - size_of::<u8>())
            ..(size_of::<T>() - size_of::<BSLPktCrc32>())])
    }
}

impl BSLMsg for BSLDeviceInfoRespPkt {}

impl BSLDeviceInfoRespPkt {
    fn validate(&self) -> Result<()> {
        self.head.validate_resp(GET_DEVICE_INFO, Self::len())?;

        if self.crc32() != self.tail {
            return Err(Error::InvalidResponse);
        }

        Ok(())
    }
}

#[derive(FromBytes, IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLCoreResp {
    head: BSLPktHead,
    msg: u8,
    tail: BSLPktCrc32,
}

impl BSLMsg for BSLCoreResp {}

impl BSLCoreResp {
    fn validate(&self) -> Result<()> {
        self.head.validate_resp(CORE_MESSAGE, Self::len())?;

        if self.crc32() != self.tail {
            return Err(Error::InvalidResponse);
        }

        if self.msg != OPERATION_SUCCESSFUL {
            return Err(Error::InvalidResponse);
        }

        Ok(())
    }
}

#[derive(IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLUnlockBslReqPkt {
    head: BSLPktHead,
    password: [u8; 32],
    tail: BSLPktCrc32,
}

impl BSLMsg for BSLUnlockBslReqPkt {}

impl BSLUnlockBslReqPkt {
    fn new(password: [u8; 32]) -> Self {
        let mut crc = crc_fast::Digest::new(CRC_ALGO);

        crc.update(&[COMMAND_UNLOCK_BOOTLOADER]);
        crc.update(&password);

        Self {
            head: BSLPktHead::new_req(Self::len(), COMMAND_UNLOCK_BOOTLOADER),
            password,
            tail: u32::try_from(crc.finalize()).unwrap().into(),
        }
    }
}

impl Default for BSLUnlockBslReqPkt {
    fn default() -> Self {
        Self::new([0xffu8; 32])
    }
}

#[derive(IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLStandaloneVerificationReqPkt {
    head: BSLPktHead,
    address: little_endian::U32,
    size: little_endian::U32,
    tail: BSLPktCrc32,
}

impl BSLMsg for BSLStandaloneVerificationReqPkt {}

impl BSLStandaloneVerificationReqPkt {
    fn new(size: u32) -> Self {
        let mut crc = crc_fast::Digest::new(CRC_ALGO);

        crc.update(&[COMMAND_STANDALONE_VERIFICATION]);
        crc.update(&0u32.to_le_bytes());
        crc.update(&size.to_le_bytes());

        Self {
            head: BSLPktHead::new_req(Self::len(), COMMAND_STANDALONE_VERIFICATION),
            address: 0.into(),
            size: size.into(),
            tail: u32::try_from(crc.finalize()).unwrap().into(),
        }
    }
}

#[derive(FromBytes, IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLStandaloneVerificationRespPkt {
    head: BSLPktHead,
    crc: little_endian::U32,
    tail: BSLPktCrc32,
}

impl BSLMsg for BSLStandaloneVerificationRespPkt {}

impl BSLStandaloneVerificationRespPkt {
    fn validate(&self) -> Result<()> {
        self.head
            .validate_resp(STANDALONE_VERIFICATION, Self::len())?;

        if self.crc32() != self.tail {
            return Err(Error::InvalidResponse);
        }

        Ok(())
    }
}

// BSL Program data request header + footer len.
const BSL_PROGRAM_DATA_REQ_LEN: usize =
    size_of::<BSLProgramDataReqHeadPkt>() + size_of::<BSLPktCrc32>();

#[derive(IntoBytes, Immutable)]
#[repr(C, packed)]
struct BSLProgramDataReqHeadPkt {
    head: BSLPktHead,
    address: little_endian::U32,
}

impl BSLMsg for BSLProgramDataReqHeadPkt {}

impl BSLProgramDataReqHeadPkt {
    fn new(data_len: u16, address: u32) -> Self {
        // Pkt len consists of header->cmd + anything before CRC32
        let len = (size_of::<u8>() + size_of::<u32>()) as u16 + data_len;
        Self {
            head: BSLPktHead::new_req(len, COMMAND_PROGRAM_DATA),
            address: address.into(),
        }
    }
}

pub(crate) struct Mspm0<S> {
    port: S,
    max_buffer_size: usize,
}

impl<S> Mspm0<S>
where
    S: std::io::Read + std::io::Write,
{
    pub(crate) fn new(mut port: S) -> Result<Self> {
        Self::connect(&mut port)?;
        let max_buffer_size = match Self::get_device_info(&mut port) {
            Ok(info) => info.bsl_max_buffer_size.into(),
            Err(err) => {
                tracing::warn!(
                    "Failed to read device info ({err}); falling back to default BSL buffer size"
                );
                DEFAULT_BSL_MAX_BUFFER_SIZE
            }
        };

        Ok(Self {
            port,
            max_buffer_size,
        })
    }

    fn wait_for_ack_inner(port: &mut S) -> Result<()> {
        let mut buf = [0u8; 1];

        port.read_exact(&mut buf)?;

        match buf[0] {
            BSL_ACK => Ok(()),
            BSL_ERROR_HEADER_INCORRECT => Err(Error::HeaderIncorrect),
            BSL_ERROR_CHECKSUM_INCORRECT => Err(Error::ChecksumIncorrect),
            BSL_ERROR_PACKET_SIZE_ZERO => Err(Error::PktSizeZero),
            BSL_ERROR_PACKET_SIZE_TOO_BIG => Err(Error::PktSize2Big),
            BSL_ERROR_UNKNOWN_BAUD_RATE => Err(Error::UnknownBaudRate),
            _ => Err(Error::Unknown),
        }
    }

    fn wait_for_ack(&mut self) -> Result<()> {
        Self::wait_for_ack_inner(&mut self.port)
    }

    fn wait_for_ack_or_core_success(&mut self) -> Result<()> {
        let mut buf = [0u8; 1];

        self.port.read_exact(&mut buf)?;

        match buf[0] {
            BSL_ACK => Ok(()),
            RESPONSE => {
                let mut rest = [0u8; size_of::<BSLCoreResp>() - 1];
                self.port.read_exact(&mut rest)?;

                let mut full = [0u8; size_of::<BSLCoreResp>()];
                full[0] = RESPONSE;
                full[1..].copy_from_slice(&rest);

                let resp =
                    BSLCoreResp::read_from_bytes(&full).map_err(|_| Error::InvalidResponse)?;
                resp.validate()
            }
            BSL_ERROR_HEADER_INCORRECT => Err(Error::HeaderIncorrect),
            BSL_ERROR_CHECKSUM_INCORRECT => Err(Error::ChecksumIncorrect),
            BSL_ERROR_PACKET_SIZE_ZERO => Err(Error::PktSizeZero),
            BSL_ERROR_PACKET_SIZE_TOO_BIG => Err(Error::PktSize2Big),
            BSL_ERROR_UNKNOWN_BAUD_RATE => Err(Error::UnknownBaudRate),
            _ => Err(Error::Unknown),
        }
    }

    fn connect(port: &mut S) -> Result<()> {
        tracing::info!("Establishing connection");

        BSLNoDataReqPkt::connection_req().write_to_io(&mut *port)?;
        Self::wait_for_ack_inner(port)
    }

    fn get_device_info(port: &mut S) -> Result<BSLDeviceInfoRespPkt> {
        tracing::info!("Getting Device Info");

        BSLNoDataReqPkt::get_device_info_req().write_to_io(&mut *port)?;
        Self::wait_for_ack_inner(&mut *port)?;

        let resp = BSLDeviceInfoRespPkt::read_from_io(port)?;
        resp.validate()?;

        Ok(resp)
    }

    pub(crate) fn unlock(&mut self) -> Result<()> {
        tracing::info!("Unlocking BSL");

        BSLUnlockBslReqPkt::default().write_to_io(&mut self.port)?;
        self.wait_for_ack()
    }

    pub(crate) fn standalone_verification(&mut self, size: u32) -> Result<little_endian::U32> {
        tracing::info!("Get current firmware CRC32");

        BSLStandaloneVerificationReqPkt::new(size).write_to_io(&mut self.port)?;
        self.wait_for_ack()?;

        let resp = BSLStandaloneVerificationRespPkt::read_from_io(&mut self.port)?;
        resp.validate()?;

        Ok(resp.crc)
    }

    pub(crate) fn start_application(&mut self) -> Result<()> {
        tracing::info!("Launch application");

        BSLNoDataReqPkt::new(COMMAND_START_APPLICATION).write_to_io(&mut self.port)?;
        self.wait_for_ack()
    }

    pub(crate) fn mass_erase(&mut self) -> Result<()> {
        tracing::info!("Perform mass erase");

        BSLNoDataReqPkt::new(COMMAND_MASS_ERASE).write_to_io(&mut self.port)?;
        self.wait_for_ack_or_core_success()
    }

    pub(crate) fn program_data_max_len(&self) -> usize {
        let max_data_len = self.max_buffer_size - BSL_PROGRAM_DATA_REQ_LEN;

        // Align to 8 bytes
        max_data_len - (max_data_len % 8)
    }

    pub(crate) fn program_data(&mut self, address: u32, data: &[u8]) -> Result<()> {
        assert!(data.len() <= self.program_data_max_len());

        let mut crc = crc_fast::Digest::new(CRC_ALGO);

        crc.update(&[COMMAND_PROGRAM_DATA]);

        BSLProgramDataReqHeadPkt::new(data.len().try_into().unwrap(), address)
            .write_to_io(&mut self.port)?;

        crc.update(&address.to_le_bytes());

        crc.update(data);
        self.port.write_all(data)?;

        let msg_crc = u32::try_from(crc.finalize()).unwrap();
        self.port.write_all(&msg_crc.to_le_bytes())?;

        self.wait_for_ack_or_core_success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_req_test() {
        let cmd = BSLNoDataReqPkt::connection_req();
        assert_eq!(
            cmd.as_bytes(),
            &[0x80, 0x01, 0x00, 0x12, 0x3A, 0x61, 0x44, 0xDE]
        );
    }

    #[test]
    fn get_info_req_test() {
        let cmd = BSLNoDataReqPkt::get_device_info_req();
        assert_eq!(
            cmd.as_bytes(),
            &[0x80, 0x01, 0x00, 0x19, 0xB2, 0xB8, 0x96, 0x49]
        );
    }

    #[test]
    fn unlock_req_test() {
        let cmd = BSLUnlockBslReqPkt::default();
        assert_eq!(
            cmd.as_bytes(),
            &[
                0x80, 0x21, 0x00, 0x21, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0xAA, 0xF0, 0x3D
            ]
        );
    }
}
