use anyhow::Result;
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// The XA ADPCM specification defines four pairs of filter values (K0, K1) as:
// Filter 0 = (0,         0)
// Filter 1 = (0.9375,    0)
// Filter 2 = (1.796875, -0.8125)
// Filter 3 = (1.53125,  -0.859375)
//
// 16-bit WAV PCM samples use a fixed-point encoding with the 6 least significant bits representing
// the decimal portion. Filter values for this encoding are derived via
//   fixed_point_filter = xa_adpcm_filter * 2^6
// and stored in these tables:
const XA_ADPCM_FILTER_COUNT: usize = 4;
const FILTER_K0: [i32; XA_ADPCM_FILTER_COUNT] = [0, 60, 115, 98];
const FILTER_K1: [i32; XA_ADPCM_FILTER_COUNT] = [0, 0, -52, -55];

// XA ADPCM samples are stored as 4-bits, and the decoder expands them to 16-bit samples by left
// shifting by the number of bits specified in sample unit's sound parameter. The maximum number
// of bits a sample can be shifted by is (16 - 4) = 12.
const MAX_SHIFT: usize = 12;


const SOUND_UNIT_SIZE: usize = 28;

const SAMPLE_MASK: i32 = 0xF;

pub(crate) const SOUND_UNIT_SAMPLES: usize = 28;
pub(crate) const SOUND_GROUP_SAMPLES: usize = SOUND_UNIT_SAMPLES * 8;
pub(crate) const ADPCM_SECTOR_SAMPLES: usize = SOUND_GROUP_SAMPLES * 18;
pub(crate) const XA_ADPCM_SECTOR_SIZE: usize = 0x914;

pub struct EncoderState {
  predictor_delayed_1: [i32; XA_ADPCM_FILTER_COUNT],
  predictor_delayed_2: [i32; XA_ADPCM_FILTER_COUNT],

  encoder_delayed_1: i32,
  encoder_delayed_2: i32,

  noise_shaper_delayed_1: i32,
  noise_shaper_delayed_2: i32,
  noise_shaper_output: i32,

  quantizer_input: i32,
  quantizer_output: i32,
}

impl EncoderState {
  fn new() -> Self {
    EncoderState {
      predictor_delayed_1: [0; XA_ADPCM_FILTER_COUNT],
      predictor_delayed_2: [0; XA_ADPCM_FILTER_COUNT],

      encoder_delayed_1: 0,
      encoder_delayed_2: 0,

      noise_shaper_delayed_1: 0,
      noise_shaper_delayed_2: 0,
      noise_shaper_output: 0,

      quantizer_input: 0,
      quantizer_output: 0,
    }
  }
}

fn encode_sound_unit(encoder_state: &mut EncoderState, samples: &[i16], output: &mut [u8]) -> u8 {
  // ---------------------------
  // Predictors

  // The predictors determine the peak value produced by each filter pair across the sound unit
  let mut peaks = [0_i32; 4];
  for filter in 0..XA_ADPCM_FILTER_COUNT {
    let k0 = FILTER_K0[filter];
    let k1 = FILTER_K1[filter];

    let mut delayed_1 = encoder_state.predictor_delayed_1[filter];
    let mut delayed_2 = encoder_state.predictor_delayed_2[filter];

    let mut peak: i32 = 0;
    
    for n in 0..SOUND_UNIT_SIZE {
      let dry_sample = i32::from(samples[n]);
      // Sample and filter values are fixed-point, so we need to shift right by 6 after multiplication
      // to renormalize the values. Add (1 << 5) before normalization to ensure normalized value is
      // rounded up rather than down.
      let feedback = (
        k0 * delayed_1 +
        k1 * delayed_2 +
        (1 << 5)
      ) >> 6;
      let sample = dry_sample - feedback;

      if sample.abs() > peak.abs() { peak = sample; }
      delayed_2 = delayed_1;
      delayed_1 = dry_sample;
    }

    encoder_state.predictor_delayed_1[filter] = delayed_1;
    encoder_state.predictor_delayed_2[filter] = delayed_2;
    peaks[filter] = peak;
  }

  // ---------------------------
  // Filter and range selection

  // The selected filter is the one that produced the lowest peak value across the sound unit
  let mut filter = 0;
  let mut lowest_peak = i32::from(i16::MIN);
  for n in 0..XA_ADPCM_FILTER_COUNT {
    let peak = peaks[n];
    if peak.abs() < lowest_peak.abs() {
      filter = n;
      lowest_peak = peak;
    }
  }
  let filter = filter;

  // Find the number of right shifts required to fit `highest_peak` in the 4-bit ADPCM sample range
  let mut shift = 0;
  if lowest_peak > 0 { 
    let max_peak_adpcm = i32::from(i16::MAX) >> MAX_SHIFT;
    while shift < MAX_SHIFT && (lowest_peak >> shift) > max_peak_adpcm { shift += 1; }
  } else {
    let min_peak_adpcm = i32::from(i16::MIN) >> MAX_SHIFT;
    while shift < MAX_SHIFT && (lowest_peak >> shift) < min_peak_adpcm { shift += 1; }
  }

  // Sample expansion algorithm for the decoder is
  //   word_value = adpcm_value * 2^(12 - R)
  // so range needs to be (12 - shift)
  let range = MAX_SHIFT - shift;

  // ---------------------------
  // Encoding

  let k0 = FILTER_K0[filter];
  let k1 = FILTER_K1[filter];
  for n in 0..SOUND_UNIT_SIZE {
    // Process sample with selected filter
    let dry_sample = i32::from(samples[n]);
    let feedback = (
      k0 * encoder_state.encoder_delayed_1 +
      k1 * encoder_state.encoder_delayed_2 +
      (1 << 5)
    ) >> 6;
    
    encoder_state.encoder_delayed_2 = encoder_state.encoder_delayed_1;
    encoder_state.encoder_delayed_1 = dry_sample;
    let filtered_sample = dry_sample - feedback;

    // Gain control
    let gain_control_input = filtered_sample - encoder_state.noise_shaper_output;
    let gain_controlled_sample = gain_control_input << range;
    encoder_state.quantizer_input = gain_controlled_sample;

    // Quantizer
    encoder_state.quantizer_output = (
      (encoder_state.quantizer_input + (1 << (MAX_SHIFT - 1))) >> MAX_SHIFT
    ).clamp(i32::from(i16::MIN) >> MAX_SHIFT, i32::from(i16::MAX) >> MAX_SHIFT);
    let encoded_sample = i8::try_from( encoder_state.quantizer_output ).unwrap();

    // Noise shaper
    let noise_shaper_input = ((encoder_state.quantizer_output << MAX_SHIFT) - encoder_state.quantizer_input) >> range;
    encoder_state.noise_shaper_delayed_2 = encoder_state.noise_shaper_delayed_1;
    encoder_state.noise_shaper_delayed_1 = noise_shaper_input;
    encoder_state.noise_shaper_output =  (
      k0 * encoder_state.noise_shaper_delayed_1 +
      k1 * encoder_state.noise_shaper_delayed_1 +
      (1 << 5)
    ) >> 6;

    // Write sample to output buffer
    let encoded_byte = encoded_sample.to_be_bytes()[0];
    output[n] = encoded_byte;
  }

  // Encode sound parameter
  let filter_byte = u8::try_from(filter).unwrap();
  let range_byte = u8::try_from(range).unwrap();

  ((filter_byte << 4) & 0xF0) + (range_byte & 0x0F)
}

fn fill_sample_buffer<R: Read>(samples: &mut[i16], rdr: &mut R) {
    for n in 0..samples.len() {
    match rdr.read_i16::<LE>() {
      Ok(sample) => samples[n] = sample,
      Err(_) => samples[n] = 0,
    }
  }
}

fn write_combined_bytes<W: Write>(b1: u8, b2: u8, output: &mut W) -> Result<()> {
  let out_byte = (b1 & 0x0F) + ((b2 & 0x0F) << 4);

  output.write_u8(out_byte)?;

  Ok(())
}

fn encode_sound_group<R: Read, W: Write>(encoder_state: &mut EncoderState, input: &mut R, output: &mut W) -> Result<()> {
  let mut pcm_samples = [0_i16; 28];
  let mut sound_unit_0 = vec![0_u8; 28];
  let mut sound_unit_1 = vec![0_u8; 28];
  let mut sound_unit_2 = vec![0_u8; 28];
  let mut sound_unit_3 = vec![0_u8; 28];
  let mut sound_unit_4 = vec![0_u8; 28];
  let mut sound_unit_5 = vec![0_u8; 28];
  let mut sound_unit_6 = vec![0_u8; 28];
  let mut sound_unit_7 = vec![0_u8; 28];

  fill_sample_buffer(&mut pcm_samples, input);
  let p0 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_0);

  fill_sample_buffer(&mut pcm_samples, input);
  let p1 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_1);

  fill_sample_buffer(&mut pcm_samples, input);
  let p2 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_2);

  fill_sample_buffer(&mut pcm_samples, input);
  let p3 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_3);

  fill_sample_buffer(&mut pcm_samples, input);
  let p4 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_4);

  fill_sample_buffer(&mut pcm_samples, input);
  let p5 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_5);

  fill_sample_buffer(&mut pcm_samples, input);
  let p6 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_6);

  fill_sample_buffer(&mut pcm_samples, input);
  let p7 = encode_sound_unit(encoder_state, &pcm_samples, &mut sound_unit_7);

  let sound_parameters = [
    p0, p1, p2, p3, p0, p1, p2, p3, p4, p5, p6, p7, p4, p5, p6, p7
  ];

  output.write_all(&sound_parameters)?;
  for k in 0..28 {
    write_combined_bytes(sound_unit_0[k], sound_unit_1[k], output)?;
    write_combined_bytes(sound_unit_2[k], sound_unit_3[k], output)?;
    write_combined_bytes(sound_unit_4[k], sound_unit_5[k], output)?;
    write_combined_bytes(sound_unit_6[k], sound_unit_7[k], output)?;
  }

  Ok(())
}

fn encode_sound_block<R: Read, W: Write>(encoder_state: &mut EncoderState, input: &mut R, output: &mut W) -> Result<()> {
  for _ in 0..18 {
    encode_sound_group(encoder_state, input, output)?;
  }  

  Ok(())
}

fn encode_sector<R: Read, W: Write>(encoder_state: &mut EncoderState, input: &mut R, output: &mut W) -> Result<()> {
  encode_sound_block(encoder_state, input, output)?;

  let zero_pad = [0_u8; 0x14];
  output.write_all(&zero_pad)?;

  Ok(())
}

pub(crate) fn encode_xa_adpcm<R: Read, W: Write>(samples_count: usize, input: &mut R, output: &mut W) -> Result<()> {
  let mut encoder_state = EncoderState::new();
  
  let mut num_sectors = samples_count / ADPCM_SECTOR_SAMPLES;
  if samples_count % ADPCM_SECTOR_SAMPLES != 0 { num_sectors += 1 }

  for _ in 0..num_sectors {
    encode_sector(&mut encoder_state, input, output)?;
  }

  Ok(())
}
