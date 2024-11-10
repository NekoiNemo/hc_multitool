use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read};

use serde_json::{json, Map, Value};

const MARKER_BOOL: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
const MARKER_INT: [u8; 4] = [0x02, 0x00, 0x00, 0x00];
const MARKER_UKN3: [u8; 4] = [0x03, 0x00, 0x00, 0x00];
const MARKER_STRING: [u8; 4] = [0x04, 0x00, 0x00, 0x00];
const MARKER_COORDS: [u8; 4] = [0x05, 0x00, 0x00, 0x00];
const MARKER_REF: [u8; 4] = [0x12, 0x00, 0x00, 0x00];
const MARKER_OBJECT: [u8; 4] = [0x14, 0x00, 0x00, 0x00];
const MARKER_ARRAY: [u8; 4] = [0x15, 0x00, 0x00, 0x00];

const MARKER_ARROBJ_START: u8 = 0x80;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: <program> <save_file_path>");
        return;
    }
    let input_path = &args[1];

    println!("Reading file {input_path}");

    let file = File::open(input_path).expect("failed to open input file");
    let mut reader = BufReader::new(file);
    let mut buf4b: [u8; 4] = [0; 4];

    // ==== Prep

    reader
        .read_exact(&mut buf4b)
        .expect("failed to read the data size");

    let mut json_save = Map::new();
    json_save.insert("version".to_string(), Value::Number(1.into()));

    // ==== Parse save data

    let save_data = read_value(&mut reader);

    // ==== Output

    json_save.insert("save_data_key".to_string(), save_data);

    let output_path = format!("{}.json", input_path.trim_end_matches(".bin"));

    println!("Converted save successfully, saving to {output_path}");

    fs::write(output_path, Value::Object(json_save).to_string()).expect("failed to write output file");
}

fn read_value(reader: &mut BufReader<File>) -> Value {
    let mut buf4b: [u8; 4] = [0; 4];

    reader
        .read_exact(&mut buf4b)
        .expect("failed to read type marker");

    match buf4b {
        MARKER_BOOL => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read bool value");

            let value = match buf4b[0] {
                0 => false,
                1 => true,
                _ => panic!("unexpected bool value: {:02X?}", buf4b),
            };

            Value::Bool(value)
        }
        MARKER_INT => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read int value");

            let value = i32::from_le_bytes(buf4b);

            Value::Number(value.into())
        }
        MARKER_UKN3 => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read next 4 bytes");

            Value::Null
        }
        MARKER_STRING => {
            let str = read_string(reader, false);

            Value::String(str)
        }
        MARKER_COORDS => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read coord x value");
            let x = f32::from_le_bytes(buf4b);

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read coord y value");
            let y = f32::from_le_bytes(buf4b);

            json!({ "x": x, "y": y })
        }
        MARKER_REF => Value::Null,
        MARKER_OBJECT => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read object length value");

            if buf4b[3] != MARKER_ARROBJ_START {
                panic!("object length value didn't end in `80` byte")
            }
            buf4b[3] = 0;

            let len = u32::from_le_bytes(buf4b);

            let mut fields = Map::with_capacity(len as usize);

            for _ in 0..len {
                let name = read_string(reader, true);

                let val = read_value(reader);

                fields.insert(name, val);
            }

            Value::Object(fields)
        }
        MARKER_ARRAY => {
            reader
                .read_exact(&mut buf4b)
                .expect("failed to read array length value");

            if buf4b[3] != MARKER_ARROBJ_START {
                panic!("array length value didn't end in `80` byte")
            }
            buf4b[3] = 0;

            let len = u32::from_le_bytes(buf4b);

            let mut values: Vec<Value> = Vec::with_capacity(len as usize);

            for _ in 0..len {
                let val = read_value(reader);
                values.push(val);
            }

            Value::Array(values)
        }
        _ => panic!("encountered unexpected type marker: {:02X?}", buf4b),
    }
}

fn read_string(reader: &mut BufReader<File>, read_type: bool) -> String {
    let mut buf4b: [u8; 4] = [0; 4];

    if read_type {
        reader
            .read_exact(&mut buf4b)
            .expect("failed to read type marker");

        if buf4b != MARKER_STRING {
            panic!("expected to read string, got type: {:02X?}", buf4b)
        }
    }

    reader
        .read_exact(&mut buf4b)
        .expect("failed to read string length value");

    let str_len = u32::from_le_bytes(buf4b);

    let mut str_bytes = vec![0; str_len as usize];
    reader
        .read_exact(&mut str_bytes)
        .expect("failed to read string");
    let str = String::from_utf8(str_bytes).expect("string was not valid UTF-8");

    let skip = (4 - str_len % 4) % 4;

    if skip != 0 {
        reader
            .read_exact(&mut vec![0; skip as usize])
            .expect("failed to skip string padding");
    }

    str
}
