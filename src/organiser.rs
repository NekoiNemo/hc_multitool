use clap::Args;
use eyre::Context;
use eyre::ContextCompat;
use eyre::Result as EResult;
use serde_json::Value;
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::BufWriter;
use std::mem::take;
use tap::Tap;

use crate::utils::{self, JArr, JObj, ObjExt, SaveDirHandler};

#[derive(Args)]
#[derive(Debug)]
pub struct Ops {
    /// Save slot number (0-3)
    save_slot: u8,
}

pub fn handler(ops: Ops, mut save_dir: SaveDirHandler) -> EResult<()> {
    log::info!("Organising various messes inside the save file");

    // ======== Read input

    let save_file = save_dir.resolve_save_slot(ops.save_slot)?;
    log::info!("Reading save file {}", save_file.display());
    let mut save_json = utils::read_json_file(&save_file).context("Failed to open save file")?;

    let save_data = save_json
        .as_object_mut()
        .context("Invalid save file: not a JSON object")?
        .get_obj_mut(utils::SAVE_DATA_KEY)?;


    // ======== Stuff

    sort_cosmetics(save_data).context("Failed to sort cosmetics")?;
    sort_furniture(save_data).context("Failed to sort furniture")?;
    deduplicate_emails(save_data).context("Failed to deduplicate emails")?;

    // ======== Write output

    let output_tmp = utils::with_added_extension(&save_file, "new");
    let output_file = File::create(&output_tmp).context("Failed to create output file")?;
    serde_json::to_writer_pretty(BufWriter::new(output_file), &save_json).context("Failed to write output JSON to file")?;

    fs::rename(&save_file, utils::with_added_extension(&save_file, "bak"))
        .context("Failed to make backup of the original save")?;
    fs::rename(&output_tmp, &save_file).context("Failed to rename output file to replace input")?;

    log::info!("Finished organising");

    Ok(())
}

fn sort_cosmetics(save_data: &mut JObj) -> EResult<()> {
    const COSMETICS_LISTS: [(&str, &str); 5] = [
        ("hairlist", "Hair"),
        ("facelist", "Face"),
        ("jewllist", "Accessory"),
        ("shirtlist", "Shirt"),
        ("jacketlist", "Jacket"),
    ];

    log::info!("Sorting wardrobe items");

    for (name, label) in COSMETICS_LISTS {
        log::info!("  Sorting {label}");

        let list = save_data.get_arr_mut(name)?;

        let sorted = list
            .iter()
            .map(|val| {
                val.as_str()
                    .with_context(|| format!("Expected a string, got: {val:#?}"))
                    .map(String::from)
            })
            .collect::<EResult<Vec<String>>>()
            .with_context(|| format!("Key {name}: failed to parse array element"))?
            .tap_mut(|list| list.sort())
            .into_iter()
            .map(Value::String)
            .collect::<JArr>();

        *list = sorted;
    }

    log::info!("Sorting wardrobe items: done");

    Ok(())
}

fn sort_furniture(save_data: &mut JObj) -> EResult<()> {
    log::info!("Sorting furniture items");

    let list = save_data.get_arr_mut("furnlist")?;

    let sorted: Vec<_> = take(list)
        .into_iter()
        .map(|val| -> EResult<(Value, FurnLabel)> {
            let name = val
                .as_object()
                .with_context(|| format!("Expected an object, got: {val:#?}"))?
                .get_str("name")?
                .to_string();

            Ok((val, FurnLabel(name)))
        })
        .collect::<EResult<Vec<_>>>()
        .context("Failed to parse furniture list")?
        .tap_mut(|vec| vec.sort_by(|(_, first), (_, second)| furn_label_cmp(first, second)))
        .into_iter()
        .map(|(val, _)| val)
        .collect();

    *list = sorted;

    log::info!("Sorting furniture items: done");

    Ok(())
}

struct FurnLabel(String);

fn furn_label_cmp(first: &FurnLabel, second: &FurnLabel) -> Ordering {
    let i1 = FURN_FIXED.iter().position(|e| e == &first.0);
    let i2 = FURN_FIXED.iter().position(|e| e == &second.0);

    match (i1, i2) {
        (Some(i1), Some(i2)) => i1.cmp(&i2),
        (Some(_), _) => Ordering::Less,
        (_, Some(_)) => Ordering::Greater,
        _ => first.0.cmp(&second.0),
    }
}

const FURN_FIXED: [&str; 2] = ["computer1", "hc_journal"];

fn deduplicate_emails(save_data: &mut JObj) -> EResult<()> {
    let mut email_ids: Vec<i64> = Vec::with_capacity(32);
    let mut removed = 0;

    let mut dedup_op = |name: &str| -> EResult<()> {
        let emails = save_data.get_arr_mut(name)?;

        // emails are stored in the same way they are shown in-game: newer first
        for i in (0..emails.len()).rev() {
            let val = &emails[i];
            let id = val
                .as_i64()
                .with_context(|| format!("Expected an int, got: {val:#?}"))?;

            if email_ids.contains(&id) {
                emails.remove(i);
                removed += 1;
            } else {
                email_ids.push(id);
            }
        }

        Ok(())
    };

    log::info!("Deduplicating emails");

    dedup_op("emailreadlist")?;
    dedup_op("emailunreadlist")?;

    if removed != 0 {
        log::info!("Removed {removed} duplicated emails");
    }

    log::info!("Deduplicating emails: done");

    Ok(())
}
