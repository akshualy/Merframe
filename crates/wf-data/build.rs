use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Deserialize;

#[derive(Deserialize)]
struct CuratedMiscItems {
    names: BTreeMap<String, String>,
    derived_catch_grades: Vec<CatchGrades>,
}

#[derive(Deserialize)]
struct CatchGrades {
    prefix: String,
    medium: String,
    large: String,
}

#[derive(Deserialize)]
struct SubsumableAbilities {
    abilities: BTreeMap<String, String>,
}

fn table<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    println!("cargo:rerun-if-changed={path}");
    let json = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    serde_json::from_str(&json).map_err(|error| format!("{path}: {error}"))
}

fn emit_names(source: &mut String, name: &str, table: &BTreeMap<String, String>) {
    let _ = writeln!(source, "pub static {name}: &[(&str, &str)] = &[");
    for (key, value) in table {
        let _ = writeln!(source, "    ({key:?}, {value:?}),");
    }
    let _ = writeln!(source, "];");
}

fn generate() -> Result<String, String> {
    let mut source = String::new();
    let misc: CuratedMiscItems = table("data/misc_items.json")?;
    if misc.names.values().any(String::is_empty) {
        return Err("data/misc_items.json: an item has an empty name".to_owned());
    }
    if misc.derived_catch_grades.iter().any(|grades| {
        grades.prefix.is_empty() || grades.medium.is_empty() || grades.large.is_empty()
    }) {
        return Err("data/misc_items.json: a catch grade entry has an empty field".to_owned());
    }
    emit_names(&mut source, "MISC_ITEM_NAMES", &misc.names);
    let _ = writeln!(source, "pub static CATCH_GRADES: &[CatchGrades] = &[");
    for grades in &misc.derived_catch_grades {
        let _ = writeln!(
            source,
            "    CatchGrades {{ prefix: {:?}, medium: {:?}, large: {:?} }},",
            grades.prefix, grades.medium, grades.large
        );
    }
    let _ = writeln!(source, "];");
    let helminth: SubsumableAbilities = table("data/helminth.json")?;
    if helminth.abilities.values().any(String::is_empty) {
        return Err("data/helminth.json: a warframe has an empty ability".to_owned());
    }
    emit_names(&mut source, "HELMINTH_ABILITIES", &helminth.abilities);
    Ok(source)
}

fn main() -> ExitCode {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("tables.rs");
    match generate().and_then(|source| {
        std::fs::write(&out, source).map_err(|error| format!("{}: {error}", out.display()))
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("embedded table does not generate: {error}");
            ExitCode::FAILURE
        }
    }
}
