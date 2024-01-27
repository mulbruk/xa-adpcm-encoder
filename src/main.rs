use anyhow::Result;
use wav::{RiffHeader, FormatChunk, DataChunk};
use std::{
  fs,
  io::{BufReader, BufWriter},
  path::PathBuf,
};

mod adpcm_encoder;
mod aiff;
mod wav;

use aiff::{AIFF, CommonChunk, APCMChunk};

fn main() -> Result<()> {
  // let mut infile = fs::File::open(
  //   PathBuf::from("/mnt/e/Temp/Tactics Ogre/CP1_0001.ACM")
  // )?;

  // let aiff = AIFF::from_reader(&mut infile)?;
  // let common = CommonChunk::from_reader(&mut infile)?;
  // let adpcm = APCMChunk::from_reader(&mut infile)?;

  // println!("{}", aiff);
  // println!("{}", common);
  // println!("{}", adpcm);

  // let mut infile = fs::File::open(
  //   PathBuf::from("/mnt/e/Temp/Tactics Ogre/test_adpcm_out.aiff")
  // )?;

  // let aiff = AIFF::from_reader(&mut infile)?;
  // let common = CommonChunk::from_reader(&mut infile)?;
  // let adpcm = APCMChunk::from_reader(&mut infile)?;

  // println!("{}", aiff);
  // println!("{}", common);
  // println!("{}", adpcm);



  let infile = fs::File::open(
    PathBuf::from("/mnt/e/Temp/Tactics Ogre/test_input_1.wav")
  )?;
  let mut rdr = BufReader::new(infile);
  let riff_header = RiffHeader::from_reader(&mut rdr)?;
  let format_chunk = FormatChunk::from_reader(&mut rdr)?;
  let data_chunk = DataChunk::from_reader(&mut rdr)?;

  println!("{:?}", riff_header);
  println!("{:?}", format_chunk);
  println!("{:?}", data_chunk);

  let num_samples = data_chunk.samples_count();

  let outfile = fs::File::create("/mnt/e/Temp/Tactics Ogre/TEST1.ACM")?;
  let mut wtr = BufWriter::new(outfile);

  aiff::write_apcm_aiff_header(num_samples, &mut wtr)?;
  adpcm_encoder::encode_xa_adpcm(num_samples, &mut rdr, &mut wtr)?;



  // let sectors = (adpcm.chunkSize - 8) / adpcm.sectorSize;
  // let raw_samples = sectors * 0x7E0 * 2;
  // let extra_samples = (raw_samples as u32) - common.numSampleFrames;
  // let extra_sample_bytes = extra_samples / 2;
  // let extra_sectors = extra_sample_bytes / 0x7E0;
  
  // let overflow_samples = extra_sample_bytes - (extra_sectors * 0x7E0);
  // let overflow_portions = overflow_samples / 112;
  // let overflow_overflow_samples = overflow_samples - (overflow_portions * 112);

  // println!("overflow_portions: {}", overflow_portions);
  // println!("overflow_samples: {}", overflow_overflow_samples);

  // let tail = extra_sectors * 0x7E0 + overflow_portions * 128 + overflow_overflow_samples;

  // println!("tail {:X}", tail);

  Ok(())
}
