use clap::Args;
use eyre::eyre;
use eyre::Context;
use eyre::Result as EResult;
use serde_json::{json, Map, Value};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::PathBuf;
use tap::Pipe;

use crate::utils;

#[derive(Args)]
#[derive(Debug)]
pub struct Ops {
    /// Path to the save file being converted
    ///
    /// Old versions of the game kept saves in the `~/.godot/app_userdata/HARDCODED`
    input_path: PathBuf,
    /// Path to write the converted save to
    ///
    /// If not set will save the output to the same dir as input file, attempting to convert its name to the new format:
    ///
    /// - savegame.bin -> savefile0.json
    /// - savegame2.bin -> savefile1.json
    /// - savegame3.bin -> savefile2.json
    /// - savegame4.bin -> savefile3.json
    ///
    /// But if input file's name didn't match the expected - will simply append `.json` to it.
    #[arg(short, long, verbatim_doc_comment)]
    output_path: Option<PathBuf>,
}

pub fn handler(ops: Ops) -> EResult<()> {
    log::info!("Converting old binary save file to new JSON format");

    let input_path = ops.input_path;

    // ======== Read input

    log::info!("Reading input file {}", input_path.display());

    let input_file = File::open(&input_path).context("Failed to open input file")?;
    let mut reader = BufReader::new(input_file);

    // ======== Convert

    read4b(&mut reader).context("Failed to read the first 4 bytes... Somehow")?;

    log::info!("Converting binary data to JSON");

    let data = read_value(&mut reader).context("Failed to read the main data of the save file")?;

    let json = json!({
        "version": 1,
        utils::SAVE_DATA_KEY: data
    });

    // ======== Write output

    let output_path = ops
        .output_path
        .or_else(|| {
            input_path
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|name| match name {
                    "savegame.bin" => Some("savefile0.json".to_string()),
                    "savegame2.bin" => Some("savefile1.json".to_string()),
                    "savegame3.bin" => Some("savefile2.json".to_string()),
                    "savegame4.bin" => Some("savefile3.json".to_string()),
                    _ => None,
                })
                .map(|new_name| input_path.with_file_name(new_name))
        })
        .unwrap_or_else(|| utils::with_added_extension(&input_path, "json"));

    log::info!("Writing output to {}", output_path.display());

    let output_file = File::create(&output_path).context("Failed to create output file")?;
    serde_json::to_writer_pretty(BufWriter::new(output_file), &json).context("Failed to write output JSON to file")?;

    log::info!("Finished save conversion");

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Type {
    Bool,
    Int,
    Unknown3,
    String,
    Coordinates,
    Reference,
    Object,
    Array,
}

impl Type {
    fn from_marker(marker: [u8; 4]) -> EResult<Type> {
        if marker[1..] != [0, 0, 0] {
            return Err(eyre!("Unexpected marker structure: {marker:02X?}"));
        }

        match marker[0] {
            0x01 => Ok(Type::Bool),
            0x02 => Ok(Type::Int),
            0x03 => Ok(Type::Unknown3),
            0x04 => Ok(Type::String),
            0x05 => Ok(Type::Coordinates),
            0x12 => Ok(Type::Reference),
            0x14 => Ok(Type::Object),
            0x15 => Ok(Type::Array),
            val => Err(eyre!("Unexpected marker value: {val:02X}")),
        }
    }

    fn read_marker(reader: &mut BufReader<File>) -> EResult<Type> {
        read4b(reader)
            .context("Failed to read marker bytes")?
            .pipe(Self::from_marker)
    }
}

fn read4b(reader: &mut BufReader<File>) -> EResult<[u8; 4]> {
    let mut buf4b: [u8; 4] = [0; 4];

    reader
        .read_exact(&mut buf4b)
        .context("Failed to read next 4 bytes")?;

    Ok(buf4b)
}

fn read_len(reader: &mut BufReader<File>, ty: Type) -> EResult<u32> {
    match ty {
        Type::String => read4b(reader)
            .context("Failed to read data length bytes")?
            .pipe(u32::from_le_bytes)
            .pipe(Ok),
        Type::Object | Type::Array => {
            let mut len_bytes = read4b(reader).context("Failed to read data length bytes")?;

            if len_bytes[3] == 0x80 {
                len_bytes[3] = 0;

                Ok(u32::from_le_bytes(len_bytes))
            } else {
                Err(eyre!(
                    "Expected last byte of object/array length to be 0x80, got: {len_bytes:02X?}"
                ))
            }
        }
        _ => unreachable!("Attempted to read length of invalid type"),
    }
}

fn read_string(reader: &mut BufReader<File>, check_marker: bool) -> EResult<String> {
    if check_marker {
        let ty = Type::read_marker(reader)?;

        if ty != Type::String {
            return Err(eyre!("Expected to read String, found marker for {ty:?}"));
        }
    }

    let str_len = read_len(reader, Type::String)?;

    let mut str_bytes = vec![0; str_len as usize];
    reader
        .read_exact(&mut str_bytes)
        .context("Failed to read string bytes")?;
    let str = String::from_utf8(str_bytes).context("Read string was not valid UTF-8")?;

    // Strings are padded to align with 4 bytes
    let skip = (4 - str_len % 4) % 4;

    if skip != 0 {
        reader
            .read_exact(&mut vec![0; skip as usize])
            .context("Failed to skip string padding")?;
    }

    Ok(str)
}

fn read_f32(reader: &mut BufReader<File>) -> EResult<f32> {
    read4b(reader)
        .context("Failed to read f32 bytes")?
        .pipe(f32::from_le_bytes)
        .pipe(Ok)
}

fn read_value(reader: &mut BufReader<File>) -> EResult<Value> {
    let ty = Type::read_marker(reader).context("Failed to read type of the value")?;

    match ty {
        Type::Bool => {
            let bytes = read4b(reader).context("Failed to read Bool bytes")?;

            let value = match bytes[0] {
                0 => false,
                1 => true,
                val => Err(eyre!("Unexpected Bool value: {val:02X?}"))?,
            };

            Ok(Value::Bool(value))
        }
        Type::Int => {
            let bytes = read4b(reader).context("Failed to read Int")?;
            let value = u32::from_le_bytes(bytes);

            Ok(Value::Number(value.into()))
        }
        Type::Unknown3 => {
            let bytes = read4b(reader).context("Failed to read 0x03 type bytes")?;

            log::warn!(
                "Encountered the 0x03 type value. Raw value: {bytes:02X?}. Not sure how to interpret so skipping"
            );

            Ok(Value::Null)
        }
        Type::String => read_string(reader, false)?.pipe(Value::String).pipe(Ok),
        Type::Coordinates => {
            let x = read_f32(reader).context("Failed to read coordinate X")?;
            let y = read_f32(reader).context("Failed to read coordinate Y")?;

            Ok(json!({ "x": x, "y": y }))
        }
        Type::Reference => {
            log::warn!("Encountered the 0x12 type value. Has no data, skipping");

            Ok(Value::Null)
        }
        Type::Object => {
            let len = read_len(reader, Type::Object).context("Failed to read field amount for object")?;
            let mut fields = Map::with_capacity(len as usize);

            for i in 0..len {
                let name = read_string(reader, true).with_context(|| format!("Failed to read {i}th field's name"))?;
                let value =
                    read_value(reader).with_context(|| format!("Failed to read value of '{name}' ({i}th field)"))?;

                if value.is_null() {
                    log::warn!("Got NULL value for {name} ({i}th field) - skipping");
                } else {
                    fields.insert(name, value);
                }
            }

            fields.sort_keys();

            Ok(Value::Object(fields))
        }
        Type::Array => {
            let len = read_len(reader, Type::Object).context("Failed to read field amount for object")?;
            let mut values: Vec<Value> = Vec::with_capacity(len as usize);

            for i in 0..len {
                let value = read_value(reader).with_context(|| format!("Failed to read {i}th element"))?;

                if value.is_null() {
                    log::warn!("Got NULL value for {i}th element - skipping");
                } else {
                    values.push(value);
                }
            }

            Ok(Value::Array(values))
        }
    }
}
