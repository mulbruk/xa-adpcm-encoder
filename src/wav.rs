use anyhow::{anyhow, Result}; 
use byteorder::{LE, ReadBytesExt};
use std::io::Read;

#[derive(Debug)]
pub(crate) struct RiffHeader {
  chunk_id: [u8; 4], // 'RIFF'
  chunk_size: u32,   // 32 + sample data size
  format: [u8; 4],   // 'WAVE'
}

impl RiffHeader {
  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id = [0_u8; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x52, 0x49, 0x46, 0x46] {
      return Err(anyhow!("Not a RIFF file"))
    }

    let chunk_size = rdr.read_u32::<LE>()?;

    let mut format = [0_u8; 4];
    rdr.read_exact(&mut format)?;
    if format != [0x57, 0x41, 0x56, 0x45] {
      return Err(anyhow!("Not a WAVE file"))
    }

    Ok(RiffHeader {
      chunk_id,
      chunk_size,
      format,
    })
  }
}

#[derive(Debug)]
pub(crate) struct FormatChunk {
  chunk_id: [u8; 4],    // 'fmt '
  chunk_size: u32,      // 16
  audio_format: u16,    // 1 (PCM)
  num_channels: u16,    // 1 (Mono)
  sample_rate: u32,     // 18900
  byte_rate: u32,       // sample_rate * num_channels * 16/8
  block_align: u16,     // 2 * 16/8
  bits_per_sample: u16, // 16
}

impl FormatChunk {
  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id = [0_u8; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x66, 0x6D, 0x74, 0x20] {
      return Err(anyhow!("Not a `fmt ` chunk"))
    }

    let chunk_size = rdr.read_u32::<LE>()?;
    
    let audio_format = rdr.read_u16::<LE>()?;
    if audio_format != 1 {
      return Err(anyhow!("Unsupported audio format: {}", audio_format))
    }
    
    let num_channels = rdr.read_u16::<LE>()?;
    if num_channels != 1 {
      return Err(anyhow!("Unsupported number of audio channels: {}", num_channels))
    }

    let sample_rate = rdr.read_u32::<LE>()?;
    if sample_rate != 18900 {
      return Err(anyhow!("Unsupported sample rate: {}", sample_rate))
    }

    let byte_rate = rdr.read_u32::<LE>()?;
    if byte_rate != (18900 * 2) {
      return Err(anyhow!("Unexpected byte rate: {}", byte_rate))
    }

    let block_align = rdr.read_u16::<LE>()?;
    if block_align != 2 {
      return Err(anyhow!("Unexpected block align: {}", block_align))
    }
    
    let bits_per_sample = rdr.read_u16::<LE>()?;
    if bits_per_sample != 16 {
      return Err(anyhow!("Unsupported number of bits per sample: {}", bits_per_sample))
    }

    Ok(FormatChunk {
      chunk_id,
      chunk_size,
      audio_format,
      num_channels,
      sample_rate,
      byte_rate,
      block_align,
      bits_per_sample,
    })
  }
}

#[derive(Debug)]
pub(crate) struct DataChunk {
  chunk_id: [u8; 4],     // 'data'
  chunk_size: u32,       // sample data size
}

impl DataChunk {
  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id = [0_u8; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x64, 0x61, 0x74, 0x61] {
      return Err(anyhow!("Not a `data` chunk: {:?}", chunk_id))
    }

    let chunk_size = rdr.read_u32::<LE>()?;

    Ok(DataChunk {
      chunk_id,
      chunk_size,
    })
  }

  // TODO temp function for testing
  pub fn samples_count(&self) -> usize {
    (self.chunk_size / 2) as usize
  }
}
