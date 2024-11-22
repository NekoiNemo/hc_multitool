# HARDCODED multitool

A tool to perform some maintenance tasks on the HARDCODED save files.

## Actions

### Save converter

A save converter from old pre-release `.bin` to new `.json` format.

Usage:

```sh
./hc_multitool convert "~/.godot/app_userdata/HARDCODED/savegame.bin"
```

This will convert the old save to new (and name it `savefile0.json`, or other save slot) in the same dir as the old save.

Optionally, output file can be specified manually:

```sh
./hc_multitool convert "~/.godot/app_userdata/HARDCODED/savegame.bin" -o "~/.local/share/godot/app_userdata/HARDCODED/savefile0-new.json"
```

When specifying the output file manually, be careful not to accidentally overwrite an existing save.

- Old save dir: `~/.godot/app_userdata/HARDCODED` (no clue what it is on Windows)
- New save dir: `~/.local/share/godot/app_userdata/HARDCODED` or `%APPDATA%\Godot\app_userdata\HARDCODED`

### List organiser

Organises various messy lists inside of a save:

- Wardrobe items: They are appended to each list, even though a lot of items are different variants of each other
and should probably go together (like same shirt with different graphics, or same haircut of different colour).
- Furniture list: Same as the above. Only *list/inventory* is affected, any placed items remain where they are.
For convenience, it keeps the PC and Journal items at the top.
- Emails: Some emails in the game can be received multiple times, each being an identical copy.
This cleans up the duplicates, keeping only the oldest copy.

Usage:

```sh
./hc_multitool organise 0
```

With `0` here being the number of the save slot (0-3).

### Outfit manager

Sadly, game doesn't currently allow saving your clothes, only changing them by hand, and as amount of clothes owned increases
(especially if you enter the item codes in terminal) - playing dress up (which is one of the main things money is spent in the game)
can become a bit tedious.

So this is where this part of the tool comes in: it allows you to extract currently worn clothes in a save and store this set in a separate file
to  then easily put it back on whenever you want (or even transfer to another save, **provided that save bought/unlocked the corresponding parts!**)

By default outfits are stored in the `outfits.json` in the game's save dir (see below about that),
but the file can be specified by hand with `--outfits-path`:

```sh
./hc_multitool outfits --outfits-path "~/hc_outfits.json" list
```

#### List outfits

```sh
./hc_multitool outfits list
```

Will simply print out all saved outfits to the terminal, like:

```
my_outfit    H:k S:z8 J:v3
outfit2      F:aa A:a S:z8 J:v3`

```

With first column being its name, and second list of parts (`H`air, `F`ace, `A`ccessory, `S`shirt and `J`acket) with their in-game values.

*Alternatively you could just, you know, open the `outfits.json` in a text editor and see for yourself.*

#### Save outfit

```sh
./hc_multitool outfits save 0 my_outfit
```

Save currently worn outfit in save slot 0 (can be 0-3) to outfits file as "my_outfit".

**The name of the outfit must be a valid JSON key, so, if you don't know what is it, just stick to alphanumerics (and start with a letter!) and using `_` in place of whitespaces.**

If you only want to save *part* of your current outfit (e.g. only shirt, accessory and jacket, but keep hair and face unaltered
when its loaded) - you can edit the `outfits.json` file to remove the corresponding properties from the outfit you've just saved.
If you then want to overwrite this outfit while keeping it partial - specify the `--partial` (or simply `-p`) argument:

```sh
./hc_multitool outfits save 0 partial_outfit --partial
```

Naturally, this does nothing if the outfit with this name doesn't exist and is saved for the first time.

#### Load outfit

```sh
./hc_multitool outfits load 0 my_outfit
```

Load the outfit "my_outfit" into the save slot 0 (can be 0-3).

Name of the outfit is optional and, if not specified, will default to, well, "default" (you can specify it by hand too, if you want).
This is an outfit consisting of parts corresponding to start-of-the-game hair and shirt, and "empty" for every other slot.

To avoid issues, this will fail if the save you're loading the outfit to doesn't own every part of the outfit (and tell you what you're missing).
If you still want to load the outfit, you can specify the `--partial` (or simply `-p`) argument:

```sh
./hc_multitool outfits load 0 not_quite_owned_outfit --partial
```

This will cause any item not owned by the current save to get skipped, and only owned items will be applied.

## Configuration

### Save location

Game keeps save files in `~/.local/share/godot/app_userdata/HARDCODED` (or `%APPDATA%\Godot\app_userdata\HARDCODED` on Windows)
and Multitool will use those paths by default. If for some reason it fails to detect them or you want to use a different path -
it can be overwritten with `--save-dir`:

```sh
./hc_multitool --save-dir "~/Documents/HARDCODED saves" organise 0
```

### Log output

This application uses [env_logger](https://crates.io/crates/env_logger) crate as logging implementation,
so the output can be configured with `RUST_LOG` environment variable (default level is `info`), e.g.:

```
# no logs
RUST_LOG=off ./hc_multitool
# logs with debug
RUST_LOG=debug ./hc_multitool
```

See [docs](https://docs.rs/env_logger/0.11.5/env_logger/#enabling-logging) for more examples.
