use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Deserialize;

#[derive(Deserialize)]
struct SolNode {
    value: String,
    #[serde(default, rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct MissionType {
    value: String,
}

#[derive(Deserialize)]
struct SortieBoss {
    name: String,
    faction: String,
}

#[derive(Deserialize)]
struct SortieModifier {
    name: String,
}

#[derive(Deserialize)]
struct ChallengeText {
    title: String,
    description: String,
}

fn table<T: for<'de> Deserialize<'de>>(path: &str) -> Result<BTreeMap<String, T>, String> {
    println!("cargo:rerun-if-changed={path}");
    let json = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    serde_json::from_str(&json).map_err(|error| format!("{path}: {error}"))
}

fn emit<T>(
    source: &mut String,
    name: &str,
    item_type: &str,
    table: &BTreeMap<String, T>,
    literal: impl Fn(&T) -> String,
) {
    let _ = writeln!(source, "pub static {name}: &[(&str, {item_type})] = &[");
    for (key, item) in table {
        let _ = writeln!(source, "    ({key:?}, {}),", literal(item));
    }
    let _ = writeln!(source, "];");
}

fn emit_nodes(source: &mut String) -> Result<(), String> {
    let nodes: BTreeMap<String, SolNode> = table("data/sol_nodes.json")?;
    if nodes.values().any(|node| node.value.is_empty()) {
        return Err("data/sol_nodes.json: a node has an empty name".to_owned());
    }
    emit(source, "SOL_NODES", "SolNode", &nodes, |node| {
        format!(
            "SolNode {{ value: {:?}, kind: {:?} }}",
            node.value, node.kind
        )
    });
    let mastery: BTreeMap<String, u32> = table("data/node_mastery.json")?;
    if let Some(node_id) = mastery.keys().find(|node_id| !nodes.contains_key(*node_id)) {
        return Err(format!(
            "data/node_mastery.json: {node_id} is not in data/sol_nodes.json"
        ));
    }
    emit(source, "NODE_MASTERY", "u32", &mastery, u32::to_string);
    let missions: BTreeMap<String, MissionType> = table("data/mission_types.json")?;
    if missions.values().any(|mission| mission.value.is_empty()) {
        return Err("data/mission_types.json: a mission type has an empty name".to_owned());
    }
    emit(source, "MISSION_TYPES", "&str", &missions, |mission| {
        format!("{:?}", mission.value)
    });
    Ok(())
}

fn emit_sorties(source: &mut String) -> Result<(), String> {
    let bosses: BTreeMap<String, SortieBoss> = table("data/sortie_bosses.json")?;
    if bosses
        .values()
        .any(|boss| boss.name.is_empty() || boss.faction.is_empty())
    {
        return Err("data/sortie_bosses.json: a boss has an empty name or faction".to_owned());
    }
    emit(source, "SORTIE_BOSSES", "SortieBoss", &bosses, |boss| {
        format!(
            "SortieBoss {{ name: {:?}, faction: {:?} }}",
            boss.name, boss.faction
        )
    });
    let modifiers: BTreeMap<String, SortieModifier> = table("data/sortie_modifiers.json")?;
    if modifiers.values().any(|modifier| modifier.name.is_empty()) {
        return Err("data/sortie_modifiers.json: a modifier has an empty name".to_owned());
    }
    emit(
        source,
        "SORTIE_MODIFIERS",
        "SortieModifier",
        &modifiers,
        |modifier| format!("SortieModifier {{ name: {:?} }}", modifier.name),
    );
    Ok(())
}

fn emit_challenges(source: &mut String) -> Result<(), String> {
    let challenges: BTreeMap<String, ChallengeText> = table("data/nightwave_challenges.json")?;
    if let Some(path) = challenges.keys().find(|path| {
        !path.starts_with("/lotus/types/challenges/seasons/") || **path != path.to_lowercase()
    }) {
        return Err(format!(
            "data/nightwave_challenges.json: {path} is not a lowercase season challenge path"
        ));
    }
    if challenges
        .values()
        .any(|challenge| challenge.title.is_empty() || challenge.description.is_empty())
    {
        return Err(
            "data/nightwave_challenges.json: a challenge has an empty title or description"
                .to_owned(),
        );
    }
    emit(
        source,
        "NIGHTWAVE_CHALLENGES",
        "ChallengeText",
        &challenges,
        |challenge| {
            format!(
                "ChallengeText {{ title: {:?}, description: {:?} }}",
                challenge.title, challenge.description
            )
        },
    );
    Ok(())
}

fn generate() -> Result<String, String> {
    let mut source = String::new();
    emit_nodes(&mut source)?;
    emit_sorties(&mut source)?;
    emit_challenges(&mut source)?;
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
