use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use crc::{Crc, CRC_64_ECMA_182};

const CRC: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
pub enum Frame {
    RunningNotificationRequest = 0x00,
    RunningNotificationResponse = 0x01,
    UnKnown = 0x16,
}
impl From<u16> for Protocol {
    fn from(value: u16) -> Protocol {
        use self::Protocol::*;

        match value {
            0x0800 => Ipv4,
            0x0806 => Arp,
            0x0842 => WakeOnLan,
        }
    }
}

impl Into<&[u8]> for Frame {
    fn into(self) -> &[u8] {
        match self {
            RunningNotificationRequest           => 0x00.as,
            RunningNotificationResponse            => 0x0806,

        }
    }
}
impl From<u8> for Frame {
    fn from(item: u8) -> Self {
        let frame = match item {
            0x00 => Frame::RunningNotificationRequest,
            0x01 => Frame::RunningNotificationResponse,
            _ => Frame::UnKnown,
        };
        frame
    }
}
impl From<Frame> for u8 {
    fn from(item: Frame) -> u8 {
        let u8_frame = match item {
            Frame::RunningNotificationRequest => 0x00,
            Frame::RunningNotificationResponse => 0x01,
            Frame::UnKnown => 0x16,
        };
        u8_frame
    }
}
impl From<&Frame> for u8 {
    fn from(item: &Frame) -> u8 {
        let u8_frame = match *item {
            Frame::RunningNotificationRequest => 0x00,
            Frame::RunningNotificationResponse => 0x01,
            Frame::UnKnown => 0x16,
        };
        u8_frame
    }
}

impl Frame {
/*    pub fn get_frame_type<B>(data: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        let mut data_ref = data.as_ref().clone();
        data_ref.advance(4 + 8);
        let frame = BigEndian::read_uint(data_ref.as_ref(), 1) as u8;
        frame.into()
    }*/

    //    impl<T: BufMut + ?Sized> BufMut for &mut T {
    //
    //    }

    pub fn pack_msg_frame(&self, data: &[u8]) -> Bytes
//    where
    //        B: Buf + BufMut,
    {
        let mut packet = BytesMut::new();
        let sum = CRC.checksum(data.as_ref());
        //        packet.reserve(data.as_ref().len() + std::mem::size_of_val(&sum) + 2 + 1);
        packet.reserve(data.as_ref().len() + std::mem::size_of_val(&sum) + 4 + 1);
        packet.put_u32(data.len() as u32);
        packet.put_u64(sum);
        packet.put_u8(self.into());
        packet.put(data.as_ref());
        packet.freeze()
    }

    // pub fn unpack_msg_frame<B>(data: &mut [u8]) -> anyhow::Result<()>
    // where
    //     B: AsRef<Bytes>,
    pub fn unpack_msg_frame(data: &mut BytesMut) -> anyhow::Result<()> {
        // let mut data_ref = data.as_ref().clone();

        let size = {
            if data.len() < 4 {
                return Err(anyhow::anyhow!("msg does not has enough 'length' byte!"));
            }
            BigEndian::read_u32(data.as_ref()) as u32
        };
        // TODO test( size + std::mem::size_of_val(size))
        if data.len() <= (size + 4) as usize {
            return Err(anyhow::anyhow!("msg is too short!"));
        }

        data.advance(std::mem::size_of_val(&size));
        let sum = BigEndian::read_u64(data.as_ref()) as u64;
        data.advance(std::mem::size_of_val(&sum));
        //        let data = data.to_vec();
        data.advance(1);

        let check = CRC.checksum(data.as_ref());
        //        dbg!(sum == check);
        println!("sum:{:?} check:{}", sum, check);
        if sum != check {
            return Err(anyhow::anyhow!("msg mismatch sum"));
        }
        Ok(())
    }
}
