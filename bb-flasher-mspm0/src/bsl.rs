use serialport::SerialPort;
use zerocopy::{FromBytes, Immutable, IntoBytes, little_endian};

use crate::{Error, Result};

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
const CORE_MESSAGE: u8 = 0x3b;

// BSL core message response msg byte
const OPERATION_SUCCESSFUL: u8 = 0x00;

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
            tail: crc32fast::hash(&[cmd]).into(),
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
    fn crc32(&self) -> u32;
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
    fn crc32(&self) -> u32 {
        crc32fast::hash(
            &self.as_bytes()[(size_of::<BSLPktHead>() - size_of::<u8>())
                ..(size_of::<T>() - size_of::<BSLPktCrc32>())],
        )
    }
}

impl BSLMsg for BSLDeviceInfoRespPkt {}

impl BSLDeviceInfoRespPkt {
    fn validate(&self) -> Result<()> {
        self.head
            .validate_resp(COMMAND_GET_DEVICE_INFO, Self::len())?;

        if self.crc32() != self.tail.into() {
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

        if self.crc32() != self.tail.into() {
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
        let mut crc = crc32fast::Hasher::new();

        crc.update(&[COMMAND_UNLOCK_BOOTLOADER]);
        crc.update(&password);

        Self {
            head: BSLPktHead::new_req(Self::len(), COMMAND_UNLOCK_BOOTLOADER),
            password,
            tail: crc.finalize().into(),
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
        let mut crc = crc32fast::Hasher::new();

        crc.update(&[COMMAND_STANDALONE_VERIFICATION]);
        crc.update(&0u32.to_le_bytes());
        crc.update(&size.to_le_bytes());

        Self {
            head: BSLPktHead::new_req(Self::len(), COMMAND_STANDALONE_VERIFICATION),
            address: 0.into(),
            size: size.into(),
            tail: crc.finalize().into(),
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
        self.head.validate_resp(CORE_MESSAGE, Self::len())?;

        if self.crc32() != self.tail.into() {
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

pub(crate) struct Mspm0<S: SerialPort> {
    port: S,
    max_buffer_size: usize,
}

impl<S> Mspm0<S>
where
    S: SerialPort,
{
    pub(crate) fn new(mut port: S) -> Result<Self> {
        Self::connect(&mut port)?;
        let info = Self::get_device_info(&mut port)?;

        Ok(Self {
            port,
            max_buffer_size: info.bsl_max_buffer_size.into(),
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
        self.wait_for_ack()?;

        let resp = BSLCoreResp::read_from_io(&mut self.port)?;
        resp.validate()
    }

    pub(crate) fn standalone_verification(&mut self, size: u32) -> Result<u32> {
        tracing::info!("Get current firmware CRC32");

        BSLStandaloneVerificationReqPkt::new(size).write_to_io(&mut self.port)?;
        self.wait_for_ack()?;

        let resp = BSLStandaloneVerificationRespPkt::read_from_io(&mut self.port)?;
        resp.validate()?;

        Ok(resp.crc.into())
    }

    pub(crate) fn start_application(&mut self) -> Result<()> {
        tracing::info!("Launch application");

        BSLNoDataReqPkt::new(COMMAND_START_APPLICATION).write_to_io(&mut self.port)?;
        self.wait_for_ack()
    }

    pub(crate) fn mass_erase(&mut self) -> Result<()> {
        tracing::info!("Perform mass erase");

        BSLNoDataReqPkt::new(COMMAND_MASS_ERASE).write_to_io(&mut self.port)?;
        self.wait_for_ack()?;

        let resp = BSLCoreResp::read_from_io(&mut self.port)?;
        resp.validate()
    }

    fn program_data_max_len(&self) -> usize {
        let max_data_len = self.max_buffer_size - BSL_PROGRAM_DATA_REQ_LEN;

        // Align to 8 bytes
        max_data_len - (max_data_len % 8)
    }

    pub(crate) fn program_data(&mut self, address: u32, data: &[u8]) -> Result<usize> {
        let data_len = std::cmp::min(self.program_data_max_len(), data.len());
        let mut crc = crc32fast::Hasher::new();

        crc.update(&[COMMAND_PROGRAM_DATA]);

        BSLProgramDataReqHeadPkt::new(data_len.try_into().unwrap(), address)
            .write_to_io(&mut self.port)?;

        crc.update(&data[..data_len]);
        self.port.write_all(&data[..data_len])?;

        let msg_crc = crc.finalize();
        self.port.write_all(&msg_crc.to_le_bytes())?;

        self.wait_for_ack()?;

        let resp = BSLCoreResp::read_from_io(&mut self.port)?;
        resp.validate()?;

        Ok(data_len)
    }
}
