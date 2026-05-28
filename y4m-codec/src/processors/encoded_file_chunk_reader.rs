use std::fs::File;
use std::io::Read;

use async_trait::async_trait;
use remotia::buffers::BytesMut;
use remotia::pipeline::PipelineHandle;
use remotia::traits::{FrameProcessor, FrameProperties, PullableFrameProperties};

use crate::{BufferType, FrameData, Stat};

const DEFAULT_CHUNK_SIZE: usize = 65536;

/// Reads an encoded bitstream from a file in fixed-size chunks and pushes each chunk
/// as an [`BufferType::EncodedPacket`] buffer into the frame data.
///
/// On end-of-file, emits a single frame marked with [`Stat::Eof`] so that downstream
/// processors (e.g. a decoder) can flush, then signals the pipeline to shut down.
pub struct EncodedFileChunkReader {
    file: File,
    chunk_size: usize,
    eof_emitted: bool,
    pipeline_handle: PipelineHandle,
}

impl EncodedFileChunkReader {
    /// Creates a new chunk reader with the default chunk size (64 KiB).
    pub fn new(file: File, pipeline_handle: PipelineHandle) -> Self {
        Self {
            file,
            chunk_size: DEFAULT_CHUNK_SIZE,
            eof_emitted: false,
            pipeline_handle,
        }
    }

    /// Creates a new chunk reader with a custom chunk size.
    pub fn with_chunk_size(file: File, chunk_size: usize, pipeline_handle: PipelineHandle) -> Self {
        Self {
            file,
            chunk_size,
            eof_emitted: false,
            pipeline_handle,
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for EncodedFileChunkReader {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        if self.eof_emitted {
            self.pipeline_handle.request_shutdown();
            return None;
        }

        let mut chunk = vec![0u8; self.chunk_size];
        match self.file.read(&mut chunk) {
            Ok(0) => {
                frame_data.set(Stat::Eof, 1);
                self.eof_emitted = true;
                Some(frame_data)
            }
            Ok(n) => {
                let mut buf = BytesMut::with_capacity(n);
                buf.extend_from_slice(&chunk[..n]);
                frame_data.push(BufferType::EncodedPacket, buf);
                Some(frame_data)
            }
            Err(e) => {
                log::error!("Read error: {:?}", e);
                self.pipeline_handle.request_shutdown();
                None
            }
        }
    }
}
