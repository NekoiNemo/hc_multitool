use std::cmp::Ordering;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};

use serde_json::Value;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: <program> <save_file_path>");
        return;
    }
    let input_path = &args[1];

    println!("Reading file {input_path}");

    let file = File::open(input_path).expect("failed to open input file");
    let mut save_json: Value = serde_json::from_reader(BufReader::new(file)).expect("failed to parse json");

    let save_base = save_json.as_object_mut().expect("not an object");
    let save_data = save_base
        .get_mut("save_data_key")
        .expect("key not found: save_data_key")
        .as_object_mut()
        .expect("key save_data_key isn't an object");

    // ======== Cosmetics lists sorting

    const COSMETICS_LISTS: [&str; 5] = ["facelist", "hairlist", "jacketlist", "jewllist", "shirtlist"];

    for name in COSMETICS_LISTS {
        let arr = save_data
            .get_mut(name)
            .unwrap_or_else(|| panic!("failed to find key: {name}"))
            .as_array_mut()
            .unwrap_or_else(|| panic!("key {name} wasn't an array"));

        arr.sort_by_cached_key(|val| {
            val.as_str()
                .unwrap_or_else(|| panic!("key {name} expected to be an array of strings"))
                .to_string()
        });
    }

    // ======== Furniture sorting

    const FURN_LIST: &str = "furnlist";

    let furn_list = save_data
        .get_mut(FURN_LIST)
        .unwrap_or_else(|| panic!("failed to find key: {FURN_LIST}"));
    let furn_list_arr = furn_list
        .as_array_mut()
        .unwrap_or_else(|| panic!("key {FURN_LIST} wasn't an array"));

    furn_list_arr.sort_by_cached_key(|val| {
        let name = val
            .as_object()
            .unwrap_or_else(|| panic!("key {FURN_LIST} expected to be an array of objects"))
            .get("name")
            .expect("furniture didn't have key: name")
            .as_str()
            .expect("furniture key name wasn't a string");

        FurnLabel(name.to_string())
    });

    // ======== Email deduping

    let mut email_ids: Vec<i64> = Vec::with_capacity(32);

    let mut email_dedup = |json_key: &str| {
        let emails = save_data
            .get_mut(json_key)
            .unwrap_or_else(|| panic!("failed to find key: {json_key}"))
            .as_array_mut()
            .unwrap_or_else(|| panic!("key {json_key} wasn't an array"));

        // emails are stored in the same way they are shown in-game: newer first
        for i in (0..emails.len()).rev() {
            let id = emails[i]
                .as_i64()
                .unwrap_or_else(|| panic!("key {json_key} expected to be an array of integers"));

            if email_ids.contains(&id) {
                emails.remove(i);
            } else {
                email_ids.push(id);
            }
        }
    };

    email_dedup("emailreadlist");
    email_dedup("emailunreadlist");

    // ======== Output

    let out_tmp = format!("{input_path}.new");
    let out_file = File::create(&out_tmp).expect("failed to create out file");
    let writer = BufWriter::new(out_file);
    serde_json::to_writer_pretty(writer, &save_json).expect("failed to write new save to file");

    fs::rename(input_path, format!("{input_path}.bak")).expect("failed to make backup of the original save");
    fs::rename(out_tmp, input_path).expect("failed to rename output file");
}

#[derive(PartialEq, Eq)]
struct FurnLabel(String);

impl PartialOrd for FurnLabel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const FURN_FIXED: [&str; 2] = ["computer1", "hc_journal"];

impl Ord for FurnLabel {
    fn cmp(&self, other: &Self) -> Ordering {
        let i1 = FURN_FIXED.iter().position(|e| e == &self.0);
        let i2 = FURN_FIXED.iter().position(|e| e == &other.0);

        match (i1, i2) {
            (Some(i1), Some(i2)) => i1.cmp(&i2),
            (Some(_), _) => Ordering::Less,
            (_, Some(_)) => Ordering::Greater,
            _ => self.0.cmp(&other.0),
        }
    }
}
