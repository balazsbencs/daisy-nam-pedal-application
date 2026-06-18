/// image_builder.rs — packs models, IRs, and presets into the QSPI binary format.
///
/// Binary layout (all little-endian) — must stay in sync with data_format.h:
///   NamDataHeader  (8 bytes):  magic(4) version(2) count(2)
///   NamDataEntry[] (48 bytes each): type(1) name(31) offset(4) length(4) sr(4) reserved(4)
///   blobs, 4 KB-aligned
///
/// NamPreset blob (74 bytes, packed): model_name(31) ir_name(31) input_gain(4)
///                                    output_volume(4) bypass(1) pad(3)
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};

const NAM_DATA_MAGIC:   u32 = 0x444D_414E; // 'NAMD'
const NAM_DATA_VERSION: u16 = 1;
const SECTOR_SIZE:      u32 = 4096;
const PARTITION_SIZE:   u32 = 6 * 1024 * 1024; // 6 MB

const ENTRY_MODEL:  u8 = 0;
const ENTRY_IR:     u8 = 1;
const ENTRY_PRESET: u8 = 2;

const NAME_LEN: usize = 31;

pub struct Blob {
    pub entry_type: u8,
    pub name:       String,
    pub data:       Vec<u8>,
    pub samplerate: u32,
}

pub struct BuiltImage {
    pub data:        Vec<u8>,
    pub entries:     Vec<(u8, String, u32, u32)>, // (type, name, offset, length)
}

fn align_up(v: u32, align: u32) -> u32 {
    (v + align - 1) & !(align - 1)
}

fn write_name(buf: &mut impl Write, name: &str) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(NAME_LEN - 1);
    buf.write_all(&bytes[..len]).unwrap();
    // zero-pad to NAME_LEN
    for _ in len..NAME_LEN {
        buf.write_u8(0).unwrap();
    }
}

fn write_entry(buf: &mut impl Write, entry_type: u8, name: &str,
               offset: u32, length: u32, samplerate: u32) {
    buf.write_u8(entry_type).unwrap();
    write_name(buf, name);
    buf.write_u32::<LittleEndian>(offset).unwrap();
    buf.write_u32::<LittleEndian>(length).unwrap();
    buf.write_u32::<LittleEndian>(samplerate).unwrap();
    buf.write_u32::<LittleEndian>(0).unwrap(); // reserved
}

pub fn pack_preset_blob(model_name: &str, ir_name: &str,
                        input_gain: f32, output_volume: f32, bypass: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(74);
    write_name(&mut buf, model_name);
    write_name(&mut buf, ir_name);
    buf.write_f32::<LittleEndian>(input_gain).unwrap();
    buf.write_f32::<LittleEndian>(output_volume).unwrap();
    buf.write_u8(if bypass { 1 } else { 0 }).unwrap();
    buf.extend_from_slice(&[0u8; 3]); // explicit padding
    debug_assert_eq!(buf.len(), 74, "NamPreset blob size mismatch");
    buf
}

pub fn build(blobs: &[Blob]) -> BuiltImage {
    let count = blobs.len() as u16;

    // Calculate where blob data starts (after header + full directory).
    // We size the directory for `count` entries even if some blobs are empty.
    let header_size: u32 = 8;
    let entry_size:  u32 = 48;
    let dir_end = header_size + entry_size * count as u32;
    let blob_area_start = align_up(dir_end, SECTOR_SIZE).max(SECTOR_SIZE);

    // Place each blob at 4 KB-aligned offsets.
    let mut offsets  = Vec::with_capacity(blobs.len());
    let mut cursor   = blob_area_start;
    for blob in blobs {
        offsets.push(cursor);
        cursor = align_up(cursor + blob.data.len() as u32, SECTOR_SIZE);
        if cursor == align_up(blob_area_start, SECTOR_SIZE) {
            cursor += SECTOR_SIZE;
        }
    }

    let total = cursor;
    let mut image = vec![0xFFu8; total as usize];

    // Write header.
    let mut h = &mut image[0..8];
    h.write_u32::<LittleEndian>(NAM_DATA_MAGIC).unwrap();
    h.write_u16::<LittleEndian>(NAM_DATA_VERSION).unwrap();
    h.write_u16::<LittleEndian>(count).unwrap();

    // Write directory entries.
    let mut entry_results = Vec::new();
    for (i, blob) in blobs.iter().enumerate() {
        let off    = offsets[i];
        let length = blob.data.len() as u32;
        let pos    = (header_size + entry_size * i as u32) as usize;
        let mut e  = &mut image[pos..pos + 48];
        write_entry(&mut e, blob.entry_type, &blob.name, off, length, blob.samplerate);
        entry_results.push((blob.entry_type, blob.name.clone(), off, length));
    }

    // Write blob data.
    for (i, blob) in blobs.iter().enumerate() {
        let off = offsets[i] as usize;
        image[off..off + blob.data.len()].copy_from_slice(&blob.data);
    }

    BuiltImage {
        data: image,
        entries: entry_results,
    }
}

pub fn partition_size() -> u32 { PARTITION_SIZE }
