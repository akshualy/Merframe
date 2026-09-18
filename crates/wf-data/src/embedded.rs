#[derive(Debug)]
pub struct CatchGrades {
    pub prefix: &'static str,
    pub medium: &'static str,
    pub large: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

pub fn lookup<'table, T>(table: &'table [(&str, T)], key: &str) -> Option<&'table T> {
    table
        .binary_search_by_key(&key, |(entry, _)| entry)
        .ok()
        .map(|index| &table[index].1)
}
