use clap::{Parser, Subcommand};
use color_eyre::eyre::Result as CEResult;
use std::path::PathBuf;

use crate::utils::SaveDirHandler;

mod converter;
mod organiser;
mod outfits;
mod utils;

#[cfg(debug_assertions)]
const LOGGING_LEVEL: &str = "info,hc_multitool";
#[cfg(not(debug_assertions))]
const LOGGING_LEVEL: &str = "info";

fn main() -> CEResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(LOGGING_LEVEL)).init();
    color_eyre::install()?;

    log::debug!("Parsing args");

    let cli = Cli::parse();
    let save_dir = SaveDirHandler::new_override(cli.save_dir);

    match cli.action {
        Command::Convert(ops) => converter::handler(ops),
        Command::Organise(ops) => organiser::handler(ops, save_dir),
        Command::Outfits(ops) => outfits::handler(ops, save_dir),
    }?;

    log::debug!("Exiting");

    Ok(())
}

#[derive(Parser)]
#[derive(Debug)]
struct Cli {
    #[command(subcommand)]
    action: Command,
    /// Override for the save data direcotry
    ///
    /// If not specified - application will attempt to locate it automatically
    #[arg(long)]
    save_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
#[derive(Debug)]
enum Command {
    /// Convert older pre-release save (.bin) to release version (.json)
    Convert(converter::Ops),
    /// Organise various messes inside of the save file
    ///
    /// Such as:
    ///
    /// - Sort equpable lists
    /// - Sort the furniture list
    /// - Deduplicate emails
    #[command(verbatim_doc_comment)]
    Organise(organiser::Ops),
    /// Manage outfits
    ///
    /// By default outfits will be stored in the `outfits.json` file next to the saves.
    /// "Name" of the outfit is the JSON key in that file.
    ///
    /// As game doesn't allow empty slots, `save` command will save each equiped item, however you can edit the outfit
    /// in the file by hand to remove any parts you don't want, in which case `load`-ing such outfit will only apply
    /// the pieces still left in
    Outfits(outfits::Ops),
}
