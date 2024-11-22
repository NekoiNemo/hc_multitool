use eyre::{eyre, Context, ContextCompat, Result as EResult};
use serde_json::{Map, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tap::{Pipe, Tap};

pub const SAVE_DATA_KEY: &str = "save_data_key";

pub fn with_added_extension(path: &Path, ext: &str) -> PathBuf {
    let new_ext = match path.extension() {
        Some(old_ext) => format!("{}.{ext}", old_ext.to_string_lossy()),
        None => ext.to_string(),
    };

    path.with_extension(new_ext)
}

pub fn read_json_file(path: &Path) -> EResult<Value> {
    log::debug!("Reading file {}", path.display());

    let file = File::open(path).with_context(|| format!("Failed to open file {}", path.display()))?;

    log::debug!("Parsing file as JSON");

    let json: Value = serde_json::from_reader(BufReader::new(file)).context("Failed to parse JSON in file")?;

    log::debug!("File was valid JSON");

    Ok(json)
}

pub struct SaveDirHandler {
    save_dir: Option<PathBuf>,
    dir_override: Option<PathBuf>,
}

impl SaveDirHandler {
    pub fn new_override(dir_override: Option<PathBuf>) -> Self {
        Self { save_dir: None, dir_override }
    }
    fn default_dir() -> EResult<PathBuf> {
        log::info!("Locating game save dir");

        let mut dir = dirs::data_dir().context("Unable to determine system's data dir")?;
        dir.push("godot/app_userdata/HARDCODED");

        if dir.exists() && dir.is_dir() {
            Ok(dir)
        } else {
            Err(eyre!(
                "Path {} doesn't exist or is not a directory",
                dir.display()
            ))
        }
    }

    fn resolve_save_dir(&self) -> EResult<PathBuf> {
        match self.dir_override.as_ref() {
            Some(dir) if !dir.is_dir() => Err(eyre!("Override path {} isn't a directory", dir.display())),
            Some(dir) => {
                log::info!("Save dir overridden to {}", dir.display());

                Ok(dir.clone())
            }
            None => Self::default_dir(),
        }
    }

    pub fn get_save_dir(&mut self) -> EResult<&Path> {
        if let Some(ref dir) = self.save_dir {
            return Ok(dir);
        }

        let dir = self
            .resolve_save_dir()
            .context("Unable to resolve game save directory")?;
        let dir = self.save_dir.insert(dir);

        Ok(dir)
    }

    pub fn resolve_save_slot(&mut self, slot: u8) -> EResult<PathBuf> {
        if slot > 3 {
            Err(eyre!("Invalid save slot {slot}, expected 0-3"))?
        }

        self.get_save_dir()?
            .to_owned()
            .tap_mut(|f| f.push(format!("savefile{slot}.json")))
            .pipe(Ok)
    }
}

pub type JObj = Map<String, Value>;
pub type JArr = Vec<Value>;

pub trait ObjExt {
    fn e_get(&self, name: &str) -> EResult<&Value>;
    fn e_get_mut(&mut self, name: &str) -> EResult<&mut Value>;

    fn get_obj(&self, name: &str) -> EResult<&JObj>;

    fn get_obj_mut(&mut self, name: &str) -> EResult<&mut JObj>;

    fn get_arr(&self, name: &str) -> EResult<&JArr>;

    fn get_arr_mut(&mut self, name: &str) -> EResult<&mut JArr>;

    fn get_str(&self, name: &str) -> EResult<&str>;
}

impl ObjExt for JObj {
    fn e_get(&self, name: &str) -> EResult<&Value> {
        self.get(name)
            .with_context(|| format!("Key {name}: not found"))
    }

    fn e_get_mut(&mut self, name: &str) -> EResult<&mut Value> {
        self.get_mut(name)
            .with_context(|| format!("Key {name}: not found"))
    }

    fn get_obj(&self, name: &str) -> EResult<&JObj> {
        self.e_get(name)?
            .as_object()
            .with_context(|| format!("Key {name}: not an object"))
    }

    fn get_obj_mut(&mut self, name: &str) -> EResult<&mut JObj> {
        self.e_get_mut(name)?
            .as_object_mut()
            .with_context(|| format!("Key {name}: not an object"))
    }

    fn get_arr(&self, name: &str) -> EResult<&JArr> {
        self.e_get(name)?
            .as_array()
            .with_context(|| format!("Key {name}: not an array"))
    }

    fn get_arr_mut(&mut self, name: &str) -> EResult<&mut JArr> {
        self.e_get_mut(name)?
            .as_array_mut()
            .with_context(|| format!("Key {name}: not an array"))
    }

    fn get_str(&self, name: &str) -> EResult<&str> {
        self.e_get(name)?
            .as_str()
            .with_context(|| format!("Key {name}: not a string"))
    }
}
