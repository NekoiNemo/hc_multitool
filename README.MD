# HARDCODED save converter

A save converter from old pre-release `.bin` to new `.json` format.

## Usage

`./hc_save_conv savegame.bin` then rename the resulting JSON file for to `savefileX.json` for the save slot you want.

- Old save dir: `~/.godot/app_userdata/HARDCODED`
- New save dir: `~/.local/share/godot/app_userdata/HARDCODED`

## Misc tools

### `hc_list_sorter`

Sorts cosmetics and furniture list in a save for easier in-game navigation.

Same usage as the converter: `./hc_list_sorter savefile0.json`. Creates a backup of the original save
