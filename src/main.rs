use std::env;
use std::fs::File;
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
    // let input_path = "savefile3.bin";

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

    std::fs::write(output_path, Value::Object(json_save).to_string()).expect("failed to write output file");
}

fn read_value(reader: &mut BufReader<File>) -> Value {
    let mut buf4b: [u8; 4] = [0; 4];

    reader
        .read_exact(&mut buf4b)
        .expect("failed to read type marker");

    match buf4b {
        MARKER_BOOL => {
            // println!("reading bool");

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read bool value");

            let value = match buf4b[0] {
                0 => false,
                1 => true,
                _ => panic!("unexpected bool value: {:02X?}", buf4b),
            };

            // println!("got value: {value}");

            Value::Bool(value)
        }
        MARKER_INT => {
            // println!("reading int");

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read int value");

            let value = u32::from_le_bytes(buf4b);

            // println!("got value: {value}");

            Value::Number(value.into())
        }
        MARKER_UKN3 => {
            // println!("encountered saveversion value, skipping next 4 bytes");

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
            // println!("reading coords");

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read coord x value");
            let x = f32::from_le_bytes(buf4b);

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read coord y value");
            let y = f32::from_le_bytes(buf4b);

            // println!("got values x: {x}, y: {y}");

            json!({ "x": x, "y": y })
        }
        MARKER_REF => {
            // println!("encountered ref, skipping assuming no data");

            Value::Null
        }
        MARKER_OBJECT => {
            // println!("reading object");

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read object length value");

            if buf4b[3] != MARKER_ARROBJ_START {
                panic!("object length value didn't end in `80` byte")
            }
            buf4b[3] = 0;

            let len = u32::from_le_bytes(buf4b);

            // println!("reading an object with {} fields", len);

            let mut fields = Map::with_capacity(len as usize);

            for _ in 0..len {
                // println!("reading object field {}/{}", i + 1, len);

                // println!("reading field name");

                let name = read_string(reader, true);

                // println!("reading field value");

                let val = read_value(reader);

                fields.insert(name, val);
            }

            Value::Object(fields)
        }
        MARKER_ARRAY => {
            // println!("reading array");

            reader
                .read_exact(&mut buf4b)
                .expect("failed to read array length value");

            if buf4b[3] != MARKER_ARROBJ_START {
                panic!("array length value didn't end in `80` byte")
            }
            buf4b[3] = 0;

            let len = u32::from_le_bytes(buf4b);

            // println!("reading an array of {} elements", len);

            let mut values: Vec<Value> = Vec::with_capacity(len as usize);

            for _ in 0..len {
                // println!("reading array elem {}/{}", i + 1, len);

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
    // println!("reading string");

    reader
        .read_exact(&mut buf4b)
        .expect("failed to read string length value");

    let str_len = u32::from_le_bytes(buf4b);

    // println!("string len: {str_len}");

    let mut str_bytes = vec![0; str_len as usize];
    reader
        .read_exact(&mut str_bytes)
        .expect("failed to read string");
    let str = String::from_utf8(str_bytes).expect("string was not valid UTF-8");

    let skip = (4 - str_len % 4) % 4;

    // println!("string bytes to skip: {skip}");

    if skip != 0 {
        reader
            .read_exact(&mut vec![0; skip as usize])
            .expect("failed to skip string padding");
    }

    // println!("got value: {str}");

    str
}
