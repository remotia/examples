use remotia::{
    buffers::BufMut,
    traits::{FrameError, FrameProperties},
};
use remotia_ffmpeg_codecs::FFMpegCodec;

use crate::types::{BufferType, Error, FrameData, Stat};

impl FFMpegCodec for FrameData {
    fn write_packet_data(&mut self, packet_data: &[u8]) {
        log::debug!("Writing {} bytes of packet data", packet_data.len());

        self.buffers
            .get_mut(&BufferType::EncodedPacketBuffer)
            .unwrap()
            .put(packet_data);
    }

    fn get_packet_data_buffer(&self) -> &[u8] {
        let packet_data = self.buffers.get(&BufferType::EncodedPacketBuffer).unwrap();
        log::debug!("Reading {} bytes of packet data", packet_data.len());
        &packet_data
    }

    fn write_decoded_buffer(&mut self, data: &[u8]) {
        log::debug!("Writing {} bytes of decoded frame buffer", data.len());

        self.buffers
            .get_mut(&BufferType::DecodedRGBAFrameBuffer)
            .unwrap()
            .put(data);
    }

    fn report_flush_error(&mut self) {
        self.report_error(Error::FlushError);
    }

    fn report_codec_error(&mut self) {
        self.report_error(Error::GenericCodecError);
    }

    fn report_decoder_drain_error(&mut self) {
        self.report_error(Error::DrainError);
    }

    fn set_frame_id(&mut self, frame_id: i64) {
        self.set(Stat::CaptureTime, frame_id as u128);
    }

    fn get_frame_id(&self) -> i64 {
        self.get(&Stat::CaptureTime).unwrap() as i64
    }
}
