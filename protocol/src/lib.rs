// pub mod frame;
// // pub mod opt;
//
use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use crc::{Crc, CRC_64_ECMA_182};
use protobuf::Message;

pub mod generated;

// use frame::Frame;

// pub async fn build_cmd(mut data:  &mut BytesMut) -> anyhow::Result<()> {
//     pack_msg_frame()
// Ok(())
// }
const CRC: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
pub async fn build_running_notification_req_cmd<'a>(data: &mut BytesMut) -> anyhow::Result<()> {
    let mut cmd = generated::notification::StatusRequest::new();
    cmd.status = generated::notification::StatusNotification::Running.into();
    let msg = cmd.write_to_bytes().unwrap();
    data.reserve(msg.len());
    data.extend_from_slice(msg.as_slice());

    Ok(())
}
pub fn pack_msg_frame(data: &[u8]) -> Bytes
//    where
//        B: Buf + BufMut,
{
    let mut packet = BytesMut::new();
    let sum = CRC.checksum(data.as_ref());
    //        packet.reserve(data.as_ref().len() + std::mem::size_of_val(&sum) + 2 + 1);
    packet.reserve(data.as_ref().len() + std::mem::size_of_val(&sum) + 4);
    packet.put_u32(data.len() as u32);
    packet.put_u64(sum);
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

    let check = CRC.checksum(data.as_ref());
    //        dbg!(sum == check);
    println!("sum:{:?} check:{}", sum, check);
    if sum != check {
        return Err(anyhow::anyhow!("msg mismatch sum"));
    }
    Ok(())
}
