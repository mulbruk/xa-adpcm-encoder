use anyhow::{anyhow, Result};
use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use extended::Extended;
use std::{
  fmt,
  io::{Read, Write},
};

use crate::adpcm_encoder::{
  ADPCM_SECTOR_SAMPLES,
  XA_ADPCM_SECTOR_SIZE,
};

#[derive(Debug)]
pub(crate) struct AIFF {
  chunk_id:   [u8; 4], // FourCC 'FORM' header
  chunk_size: i32,     // 4 (form type) + [8 + 18 (common chunk)] + [8 + 8 + audio_data_length bytes (ADPCM chunk)]
  form_type:  [u8; 4], // 'AIFF'
}

#[allow(non_snake_case)]
impl AIFF {
  fn new(adpcm_data_size: i32) -> Self {
    AIFF {
      chunk_id: [0x46, 0x4F, 0x52, 0x4D],
      chunk_size: 4 + 8 + 18 + 8 + 8 + adpcm_data_size,
      form_type: [0x41, 0x49, 0x46, 0x46],
    }
  }

  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id: [u8; 4] = [0; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x46, 0x4F, 0x52, 0x4D] {
      return Err(anyhow!("Not a FORM chunk: {:?}", chunk_id))
    }
    
    let chunk_size = rdr.read_i32::<BE>()?;

    let mut form_type: [u8; 4] = [0; 4];
    rdr.read_exact(&mut form_type)?;
    if form_type != [0x41, 0x49, 0x46, 0x46] {
      return Err(anyhow!("Not an AIFF form type: {:?}", form_type))
    }

    Ok(AIFF { chunk_id, chunk_size, form_type })
  }

  fn to_writer<W: Write>(&self, wtr: &mut W) -> Result<()> {
    wtr.write_all(&self.chunk_id)?;
    wtr.write_i32::<BE>(self.chunk_size)?;
    wtr.write_all(&self.form_type)?;

    Ok(())
  }
}

impl fmt::Display for AIFF {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "AIFF {{ ")?;
    write!(f, "chunk_id: {}, ", String::from_utf8_lossy(&self.chunk_id))?;
    write!(f, "chunk_size: {}, ", self.chunk_size)?;
    write!(f, "form_type: {} ", String::from_utf8_lossy(&self.form_type))?;
    write!(f, "}}") 
  }
}

#[derive(Debug)]
pub(crate) struct CommonChunk {
  chunk_id: [u8; 4], // 'COMM'
  chunk_size: i32,

  num_channels: i16,
  num_sample_frames: u32,
  sample_size: i16,
  sample_rate: Extended,
}

impl CommonChunk {
  fn new(samples_count: u32) -> Self {
    CommonChunk {
      chunk_id: [0x43, 0x4F, 0x4D, 0x4D],
      chunk_size: 18,

      num_channels: 1,
      num_sample_frames: samples_count,
      sample_size: 4,
      sample_rate: Extended::try_from(18900).unwrap(),
    }
  }

  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id: [u8; 4] = [0; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x43, 0x4F, 0x4D, 0x4D] {
      return Err(anyhow!("Not a COMM chunk: {:?}", chunk_id))
    }
    let chunk_size = rdr.read_i32::<BE>()?;

    let num_channels = rdr.read_i16::<BE>()?;
    let num_sample_frames = rdr.read_u32::<BE>()?;
    let sample_size = rdr.read_i16::<BE>()?;
    let mut sample_rate_bytes:[u8; 10] = [0; 10];
    rdr.read_exact(&mut sample_rate_bytes)?;
    let sample_rate = Extended::from_be_bytes(sample_rate_bytes);

    Ok(CommonChunk{
      chunk_id,
      chunk_size,

      num_channels,
      num_sample_frames,
      sample_size,
      sample_rate,
    })
  }

  fn to_writer<W: Write>(&self, wtr: &mut W) -> Result<()> {
    wtr.write_all(&self.chunk_id)?;
    wtr.write_i32::<BE>(self.chunk_size)?;

    wtr.write_i16::<BE>(self.num_channels)?;
    wtr.write_u32::<BE>(self.num_sample_frames)?;
    wtr.write_i16::<BE>(self.sample_size)?;
    wtr.write_all(&self.sample_rate.to_be_bytes())?;

    Ok(())
  }
}

impl fmt::Display for CommonChunk {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "CommonChunk {{ ")?;
    write!(f, "chunk_id: {}, ", String::from_utf8_lossy(&self.chunk_id))?;
    write!(f, "chunk_size: {}, ", self.chunk_size)?;
    
    write!(f, "num_channels: {}, ", self.num_channels)?;
    write!(f, "num_sample_frames: {}, ", self.num_sample_frames)?;
    write!(f, "sample_size: {}, ", self.sample_size)?;
    write!(f, "sample_rate: {} ", self.sample_rate.to_f64())?;
    write!(f, "}}") 
  }
}

#[derive(Debug)]
pub(crate) struct APCMChunk {
  chunk_id: [u8; 4], // "APCM"
  chunk_size: i32,

  unknown: i32,
  sector_size: i32,
  // XA-ADPCM sectors
}

impl APCMChunk {
  fn new(adpcm_data_size: i32) -> Self {
    APCMChunk {
      chunk_id: [0x41, 0x50, 0x43, 0x4D],
      chunk_size: 8 + adpcm_data_size,

      unknown: 0,
      sector_size: 0x914,
    }
  }

  pub fn from_reader<R: Read>(rdr: &mut R) -> Result<Self> {
    let mut chunk_id: [u8; 4] = [0; 4];
    rdr.read_exact(&mut chunk_id)?;
    if chunk_id != [0x41, 0x50, 0x43, 0x4D] {
      return Err(anyhow!("Not an APCM chunk: {:?}", chunk_id))
    }
    let chunk_size = rdr.read_i32::<BE>()?;
    
    let unknown = rdr.read_i32::<BE>()?;
    let sector_size = rdr.read_i32::<BE>()?;

    Ok(APCMChunk { chunk_id, chunk_size, unknown, sector_size })
  }

  fn to_writer<W: Write>(&self, wtr: &mut W) -> Result<()> {
    wtr.write_all(&self.chunk_id)?;
    wtr.write_i32::<BE>(self.chunk_size)?;

    wtr.write_i32::<BE>(self.unknown)?;
    wtr.write_i32::<BE>(self.sector_size)?;

    Ok(())
  }
}

impl fmt::Display for APCMChunk {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "APCMChunk {{ ")?;
    write!(f, "chunk_id: {}, ", String::from_utf8_lossy(&self.chunk_id))?;
    write!(f, "chunk_size: {} ", self.chunk_size)?;
    write!(f, "unknown: {} ", self.unknown)?;
    write!(f, "sector_size: {} ", self.sector_size)?;
    write!(f, "}}") 
  }
}

pub(crate) fn write_apcm_aiff_header<W: Write>(num_samples: usize, wtr: &mut W) -> Result<()> {
  let mut num_sectors = num_samples / ADPCM_SECTOR_SAMPLES;
  if num_samples % ADPCM_SECTOR_SAMPLES != 0 { num_sectors += 1 }
  let num_sectors = num_sectors + 3; // Three blank sectors at start

  let num_samples = num_samples + 3 * ADPCM_SECTOR_SAMPLES;
  
  let data_size = i32::try_from(num_sectors * XA_ADPCM_SECTOR_SIZE)?;
  let num_samples = u32::try_from(num_samples)?;

  let aiff = AIFF::new(data_size);
  let comm = CommonChunk::new(num_samples);
  let apcm = APCMChunk::new(data_size);

  aiff.to_writer(wtr)?;
  comm.to_writer(wtr)?;
  apcm.to_writer(wtr)?;

  println!("");
  println!("{}", aiff);
  println!("{}", comm);
  println!("{}", apcm);

  Ok(())
}
