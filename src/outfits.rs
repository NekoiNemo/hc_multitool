use clap::{Args, Subcommand};
use eyre::Context;
use eyre::Result as EResult;
use eyre::{eyre, ContextCompat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tap::Tap;

use crate::utils::{self, ObjExt, SaveDirHandler};

#[derive(Args)]
#[derive(Debug)]
pub struct Ops {
    /// Outfits file path
    ///
    /// Defaults to `outfits.json` in the same directory as the input file
    #[arg(long)]
    outfits_path: Option<PathBuf>,

    #[command(subcommand)]
    action: Cmd,
}

#[derive(Subcommand)]
#[derive(Debug)]
enum Cmd {
    /// List saved outfits
    List,
    /// Save currently worn outfit
    Save {
        /// Save slot number (0-3)
        save_slot: u8,
        /// Name of the outfit (must be a valid JSON key)
        outfit: String,
        /// Only save slots that already defined for outfit
        ///
        /// Ignored when saving a new outfit
        #[arg(short = 'p', long)]
        partial: bool,
    },
    /// Load outfit into the save file
    ///
    /// Save file must have necessary items for outfit to be loaded
    Load {
        /// Save slot number (0-3)
        save_slot: u8,
        /// Name of the outfit
        #[arg(default_value = "default")]
        outfit: String,
        /// Attempt partial loading of the outfit
        ///
        /// If save doesn't have all the necessary items - still attempt to put on items that are there,
        /// instead of returning an error
        #[arg(short = 'p', long)]
        partial: bool,
    },
}

pub fn handler(ops: Ops, mut save_dir: SaveDirHandler) -> EResult<()> {
    log::info!("Working with outfits");

    let outfits_file = if let Some(path) = ops.outfits_path {
        path
    } else {
        save_dir
            .get_save_dir()
            .context("Save dir not found and no custom path to outfits file was provided")?
            .to_owned()
            .tap_mut(|p| p.push("outfits.json"))
    };

    log::info!("Using outfit file: {}", outfits_file.display());

    match ops.action {
        Cmd::List => list_outfits(&outfits_file).context("Failed to list outfits")?,
        Cmd::Save { save_slot, outfit, partial } => {
            save_outfit(&outfits_file, outfit, &mut save_dir, save_slot, partial)
                .context("Failed to save the outfit")?
        }
        Cmd::Load { save_slot, outfit, partial } => {
            load_outfit(&outfits_file, &outfit, &mut save_dir, save_slot, partial)
                .context("Failed to load the outfit")?
        }
    }

    Ok(())
}

fn list_outfits(outfits_path: &Path) -> EResult<()> {
    let storage = read_outfits(outfits_path, false)?;

    storage
        .outfits
        .iter()
        .for_each(|(name, outfit)| println!("{name}\t{outfit}"));

    Ok(())
}

fn save_outfit(
    outfits_path: &Path,
    outfit_name: String,
    save_dir: &mut SaveDirHandler,
    save_slot: u8,
    partial: bool,
) -> EResult<()> {
    log::info!("Saving outfit");

    if outfit_name == "default" {
        return Err(eyre!("Name \"default\" is reserved for starting outfit"));
    }

    // ======== Read input

    let save_file = save_dir.resolve_save_slot(save_slot)?;
    log::info!("Reading save file {save_slot}");
    let save_json = utils::read_json_file(&save_file).context("Failed to open save file")?;

    let save_data = save_json
        .as_object()
        .context("Invalid save file: not a JSON object")?
        .get_obj(utils::SAVE_DATA_KEY)?;

    let mut storage = read_outfits(outfits_path, false)?;
    let existing = storage.outfits.get(&outfit_name);

    // ======== Getting outfit

    let get_part = |name: &str, label: &str, field: fn(&Outfit) -> Option<&str>| -> EResult<Option<String>> {
        let value = save_data
            .get_str(name)
            .with_context(|| format!("Failed to get {label}"))?;

        let out = if !partial || existing.is_none() || existing.and_then(field).is_some() {
            log::info!("{label} value: \"{value}\"");
            Some(value.to_string())
        } else {
            log::info!("{label} value: \"{value}\" (skipping)");
            None
        };

        Ok(out)
    };

    let hair = get_part("hairon", "Hair", |e| e.hair.as_deref())?;
    let face = get_part("faceon", "Face", |e| e.face.as_deref())?;
    let accessory = get_part("jewlon", "Accessory", |e| e.accessory.as_deref())?;
    let shirt = get_part("shirton", "Shirt", |e| e.shirt.as_deref())?;
    let jacket = get_part("jacketon", "Jacket", |e| e.jacket.as_deref())?;

    let outfit = Outfit { hair, face, accessory, shirt, jacket };

    log::info!("Saved the outfit \"{outfit_name}\": {outfit}");

    storage.outfits.insert(outfit_name, outfit);

    // ======== Write output

    let output_file = File::create(outfits_path).context("Failed to write to outfits file")?;
    serde_json::to_writer_pretty(BufWriter::new(output_file), &storage)
        .context("Failed to write output JSON to file")?;

    log::info!("Saved outfits file");

    Ok(())
}

fn load_outfit(
    outfits_path: &Path,
    outfit_name: &str,
    save_dir: &mut SaveDirHandler,
    save_slot: u8,
    partial: bool,
) -> EResult<()> {
    log::info!("Loading outfit");

    // ======== Read input

    let save_file = save_dir.resolve_save_slot(save_slot)?;
    log::info!("Reading save file {}", save_file.display());
    let mut save_json = utils::read_json_file(&save_file).context("Failed to open save file")?;

    let save_data = save_json
        .as_object_mut()
        .context("Invalid save file: not a JSON object")?
        .get_obj_mut(utils::SAVE_DATA_KEY)?;

    let outfit = if outfit_name == "default" {
        log::info!("Using default outfit");

        Outfit::default()
    } else {
        read_outfits(outfits_path, false)?
            .outfits
            .remove(outfit_name)
            .ok_or_else(|| eyre!("Outfit \"{outfit_name}\" not found"))?
    };

    // ======== Setting outfit

    let mut set_part = |name: &str, list_name: &str, label: &str, value: Option<String>| -> EResult<()> {
        let Some(value) = value else {
            log::info!("{label}: skip");
            return Ok(());
        };

        let owned = save_data
            .get_arr(list_name)?
            .iter()
            .map(|val| {
                val.as_str()
                    .with_context(|| format!("Expected a string, got: {val:#?}"))
                    .map(String::from)
            })
            .collect::<EResult<Vec<String>>>()
            .with_context(|| format!("Key {name}: failed to parse array element"))?
            .into_iter()
            .any(|val| val == value);

        if !owned {
            if partial {
                log::warn!("{label}: value \"{value}\" is not owned, skipping");
                return Ok(());
            } else {
                return Err(eyre!("{label}: value \"{value}\" is not owned"));
            }
        }

        log::info!("{label}: setting value \"{value}\"");
        save_data.insert(name.to_string(), Value::String(value));

        Ok(())
    };

    set_part("hairon", "hairlist", "Hair", outfit.hair)?;
    set_part("faceon", "facelist", "Face", outfit.face)?;
    set_part("jewlon", "jewllist", "Accessory", outfit.accessory)?;
    set_part("shirton", "shirtlist", "Shirt", outfit.shirt)?;
    set_part("jacketon", "jacketlist", "Jacket", outfit.jacket)?;

    // ======== Write output

    let output_tmp = utils::with_added_extension(&save_file, "new");
    let output_file = File::create(&output_tmp).context("Failed to create output file")?;
    serde_json::to_writer_pretty(BufWriter::new(output_file), &save_json)
        .context("Failed to write output JSON to file")?;

    fs::rename(&save_file, utils::with_added_extension(&save_file, "bak"))
        .context("Failed to make backup of the original save")?;
    fs::rename(&output_tmp, &save_file).context("Failed to rename output file to replace input")?;

    log::info!("Finished loading outfit");

    Ok(())
}

fn read_outfits(path: &Path, require: bool) -> EResult<OutfitsStorage> {
    if !path.exists() {
        if require {
            return Err(eyre!("Outfits file doesn't exist"));
        } else {
            log::info!("Outfits file doesn't exist");

            return Ok(OutfitsStorage { outfits: HashMap::new() });
        }
    }

    log::info!("Reading outfits");

    let json = utils::read_json_file(path).context("Failed to read outfits file")?;
    let storage = serde_json::from_value::<OutfitsStorage>(json).context("Failed to read outfit file contents")?;

    log::debug!("Found {} outfits", storage.outfits.len());

    Ok(storage)
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct Outfit {
    #[serde(skip_serializing_if = "Option::is_none")]
    hair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    face: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accessory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shirt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jacket: Option<String>,
}

impl Outfit {
    fn default() -> Self {
        Self {
            hair: Some("a".to_string()),
            face: Some("aa".to_string()),
            accessory: Some("a".to_string()),
            shirt: Some("a".to_string()),
            jacket: Some("a".to_string()),
        }
    }
}

impl Display for Outfit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        let mut wrt = |label: &str, val: Option<&str>| -> std::fmt::Result {
            if let Some(val) = val {
                if !first {
                    f.write_char(' ')?;
                }

                first = false;
                f.write_str(label)?;
                f.write_char(':')?;
                f.write_str(val)?;
            }

            Ok(())
        };

        wrt("H", self.hair.as_deref())?;
        wrt("F", self.face.as_deref())?;
        wrt("A", self.accessory.as_deref())?;
        wrt("S", self.shirt.as_deref())?;
        wrt("J", self.jacket.as_deref())?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct OutfitsStorage {
    outfits: HashMap<String, Outfit>,
}
