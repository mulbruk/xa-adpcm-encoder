use anyhow::{anyhow, Context, Result};
use wav::{RiffHeader, FormatChunk, DataChunk};
use std::{
  cmp::min, fs::{self, File}, io::{BufReader, BufWriter, Chain, Cursor, Read}, path::{Path, PathBuf}
};

mod adpcm_encoder;
mod aiff;
mod wav;

use aiff::{AIFF, CommonChunk, APCMChunk};

struct ZeroReader {
  index: usize,
  size:  usize,
}

impl ZeroReader {
  fn new(size: usize) -> Self {
    ZeroReader { index: 0, size }
  }
}
impl Read for ZeroReader {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let limit = min(buf.len(), self.size - self.index);

    for n in 0..limit { buf[n] = 0 ; }
    self.index += limit;
    
    Ok(limit)
  }
}

const WAV_SAMPLE_SIZE_BYTES: usize = 2;
const WAV_SAMPLE_RATE: usize = 18_900;

const INTERFILE_DELAY_DIVISOR: usize = 2;

const INTERFILE_SAMPLES: usize = WAV_SAMPLE_RATE / INTERFILE_DELAY_DIVISOR;
const INTERFILE_BYTES: usize = INTERFILE_SAMPLES * 2;

fn prep_input_reader(paths: Vec<PathBuf>) -> Result<(usize, Box<dyn Read>)> {
  if paths.len() == 0 {
    return Err(anyhow!("No input file paths provided"))
  }

  if paths.len() == 1 {
    let infile = fs::File::open(&paths[0])?;
    let mut rdr = BufReader::new(infile);
    let riff_header = RiffHeader::from_reader(&mut rdr)?;
    let format_chunk = FormatChunk::from_reader(&mut rdr)?;
    let data_chunk = DataChunk::from_reader(&mut rdr)?;

    return Ok((data_chunk.samples_count(), Box::new(rdr)))
  }

  // Make the buffer big to minimize reallocations
  let mut buf = Vec::with_capacity(8 * 1024 * 1024);
  let mut samples_count = 0;
  
  for (n, path) in paths.iter().enumerate() {
    println!("reading file {}", path.to_string_lossy());
    let infile = fs::File::open(path)?;
    let mut rdr = BufReader::new(infile);
    let riff_header = RiffHeader::from_reader(&mut rdr)?;
    let format_chunk = FormatChunk::from_reader(&mut rdr)?;
    let data_chunk = DataChunk::from_reader(&mut rdr)?;

    samples_count += data_chunk.samples_count();

    rdr.read_to_end(&mut buf)?;

    if n != (paths.len() - 1) {
      println!("reading zeroes");
      let mut zeroes = ZeroReader::new(INTERFILE_BYTES);
      zeroes.read_to_end(&mut buf)?;

      samples_count += INTERFILE_SAMPLES;
    }
  }

  let rdr = Cursor::new(buf);
  Ok((samples_count, Box::new(rdr)))
}

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



  // let infile = fs::File::open(
  //   PathBuf::from("/mnt/e/Temp/Tactics Ogre/test_input_1.wav")
  // )?;
  // let mut rdr = BufReader::new(infile);
  // let riff_header = RiffHeader::from_reader(&mut rdr)?;
  // let format_chunk = FormatChunk::from_reader(&mut rdr)?;
  // let data_chunk = DataChunk::from_reader(&mut rdr)?;

  // println!("{:?}", riff_header);
  // println!("{:?}", format_chunk);
  // println!("{:?}", data_chunk);

  // let num_samples = data_chunk.samples_count();

  let infiles = [
    (0x01, vec!["SCENARIO_C1_001_001_00.wav"]),
    (0x02, vec!["SCENARIO_C1_001_002_00.wav"]),
    (0x03, vec!["SCENARIO_C1_001_003_00.wav"]),
    (0x04, vec!["SCENARIO_C1_001_004_00.wav"]),
    (0x05, vec!["SCENARIO_C1_001_005_00.wav"]),
    (0x06, vec!["SCENARIO_C1_001_006_00.wav"]),
    (0x07, vec!["SCENARIO_C1_001_007_00.wav"]),
    (0x08, vec!["SCENARIO_C1_001_008_00.wav"]),
    (0x09, vec!["SCENARIO_C1_002_001_00.wav"]),
    
    (0x0A, vec!["SCENARIO_C1_002_002_00.wav"]),
    (0x0B, vec!["SCENARIO_C1_002_002_01.wav"]),
    (0x0C, vec!["SCENARIO_C1_002_002_02.wav"]),
    // (0x0C, vec!["SCENARIO_C1_002_002_00.wav", "SCENARIO_C1_002_002_01.wav", "SCENARIO_C1_002_002_02.wav"]),
    
    (0x0D, vec!["SCENARIO_C1_002_003_00.wav"]),
    (0x0E, vec!["SCENARIO_C1_002_004_00.wav"]),
    (0x0F, vec!["SCENARIO_C1_002_004_01.wav", "SCENARIO_C1_002_004_02.wav"]),
    // (0x0F, vec!["SCENARIO_C1_002_004_00.wav", "SCENARIO_C1_002_004_01.wav", "SCENARIO_C1_002_004_02.wav"]),
    (0x10, vec!["SCENARIO_C1_002_005_00.wav"]),
    (0x11, vec!["SCENARIO_C1_002_006_00.wav"]),
    (0x12, vec!["SCENARIO_C1_003_001_00.wav"]),
    (0x13, vec!["SCENARIO_C1_003_002_00.wav"]),
    (0x14, vec!["SCENARIO_C1_003_003_00.wav"]),
    (0x15, vec!["SCENARIO_C1_003_004_00.wav"]),
    (0x16, vec!["SCENARIO_C1_003_005_00.wav"]),
    (0x17, vec!["SCENARIO_C1_003_006_00.wav"]),

    (0x18, vec!["SCENARIO_C1_003_007_00.wav"]),
    (0x19, vec!["SCENARIO_C1_003_008_00.wav"]),
    (0x1A, vec!["SCENARIO_C1_003_009_00.wav"]),
    (0x1B, vec!["SCENARIO_C1_003_010_00.wav"]),
    (0x1C, vec!["SCENARIO_C1_003_011_00.wav"]),
    (0x1D, vec!["SCENARIO_C1_003_012_00.wav"]),
    (0x1E, vec!["SCENARIO_C1_003_013_00.wav"]),
    (0x1F, vec!["SCENARIO_C1_003_014_00.wav"]),
    (0x20, vec!["SCENARIO_C1_003_015_00.wav"]),
    (0x21, vec!["SCENARIO_C1_003_016_00.wav"]),

    (0x22, vec!["SCENARIO_C1_003_017_00.wav"]),
    (0x23, vec!["SCENARIO_C1_003_018_00.wav"]),
    (0x24, vec!["SCENARIO_C1_003_019_00.wav"]),
    (0x25, vec!["SCENARIO_C1_003_020_00.wav"]),
    (0x26, vec!["SCENARIO_C1_003_021_00.wav"]),
    (0x27, vec!["SCENARIO_C1_003_022_00.wav"]),
    (0x28, vec!["SCENARIO_C1_004_001_00.wav"]),
    (0x29, vec!["SCENARIO_C1_004_002_00.wav"]),
    (0x2A, vec!["SCENARIO_C1_004_002_01.wav"]),
    (0x2B, vec!["SCENARIO_C1_004_003_00.wav"]),
    
    // (0x27, vec![""]),
    // (0x28, vec![""]),
    // (0x29, vec![""]),
  ];

  // let infiles = vec![
  //   PathBuf::from("/mnt/e/Temp/Tactics Ogre/SCENARIO_C1_012_001_00.wav"),
  //   PathBuf::from("/mnt/e/Temp/Tactics Ogre/SCENARIO_C1_012_001_01.wav"),
  // ];

  for (n, base_paths) in infiles {
    let paths: Vec<PathBuf> = base_paths.iter().map(|filename| ["/mnt/e/Temp/Tactics Ogre/", filename].iter().collect()).collect();

    let (num_samples, mut rdr) = prep_input_reader(paths)?;

    let outfile = fs::File::create(format!("/mnt/e/Temp/Tactics Ogre/CP1_{:0>4}.ACM", n))?;
    let mut wtr = BufWriter::new(outfile);

    aiff::write_apcm_aiff_header(num_samples, &mut wtr)?;
    adpcm_encoder::encode_xa_adpcm(num_samples, &mut rdr, &mut wtr)?;
  }


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
