use wf_mem::{GAME_PROCESS, MemoryReader, Region, read_u32_le, read_u64_le};

use crate::Result;
use crate::chunks::{to_u64, to_usize};
use crate::roots::{image_data, in_heap, words};

const STRING_TAG: u8 = 6;
const STRING_HEADER: usize = 24;
const TABLE_TAG: u32 = 7;
const FUNCTION_TAG: u32 = 8;
const THREAD_TAG: u8 = 10;
const UPVALUE_TAG: u32 = 13;
const KEY_TAG_MASK: u32 = 0xF;

const TABLE_HEADER: usize = 0x30;
const TABLE_NODE_BITS: usize = 6;
const TABLE_ARRAY_SIZE: usize = 8;
const TABLE_ARRAY: usize = 0x18;
const TABLE_NODES: usize = 0x20;
const NODE: usize = 32;
const NODE_KEY: usize = 16;
const SLOT_TAG: usize = 12;
const MAX_NODE_BITS: u8 = 20;
const MAX_ARRAY: u32 = 1 << 20;
const MAX_STRING: usize = 1 << 16;

const CLOSURE_HEADER: usize = 8;
const CLOSURE_SLOTS: u64 = 0x20;
const CLOSURE_NATIVE: usize = 3;
const CLOSURE_UPVALUES: usize = 4;
const UPVALUE_SLOT: u64 = 8;

const THREAD_FIELDS: usize = 0x100;
const GLOBAL_STATE_FIELDS: usize = 0x1000;
const POINTER: usize = 8;
const SHARED_KEY_TAG: u32 = 1;
const SHARED_KEY: u32 = 0x9828_c6d9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaTable(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaFunction(u64);

#[derive(Debug, Clone, PartialEq)]
pub enum LuaValue {
    Text(String),
    Table(LuaTable),
    Function(LuaFunction),
    Other,
}

impl LuaValue {
    pub fn text(self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn table(self) -> Option<LuaTable> {
        match self {
            Self::Table(table) => Some(table),
            _ => None,
        }
    }

    pub fn function(self) -> Option<LuaFunction> {
        match self {
            Self::Function(function) => Some(function),
            _ => None,
        }
    }
}

fn slot_tag(slot: &[u8]) -> Option<u32> {
    read_u32_le(slot, SLOT_TAG)
}

fn read_text<R: MemoryReader + ?Sized>(reader: &R, object: u64) -> Option<String> {
    let mut header = [0u8; STRING_HEADER];
    reader.read_exact(object, &mut header).ok()?;
    if header[0] != STRING_TAG {
        return None;
    }
    let len = usize::try_from(read_u32_le(&header, STRING_HEADER - 4)?).ok()?;
    if len > MAX_STRING {
        return None;
    }
    let text = reader
        .read_vec(object + to_u64(STRING_HEADER).ok()?, len)
        .ok()?;
    String::from_utf8(text).ok()
}

fn value<R: MemoryReader + ?Sized>(reader: &R, slot: &[u8]) -> Option<LuaValue> {
    let payload = read_u64_le(slot, 0)?;
    Some(match slot_tag(slot)? {
        0 => return None,
        tag if tag == u32::from(STRING_TAG) => LuaValue::Text(read_text(reader, payload)?),
        TABLE_TAG => LuaValue::Table(LuaTable(payload)),
        FUNCTION_TAG => LuaValue::Function(LuaFunction(payload)),
        UPVALUE_TAG => {
            let slot = reader.read_u64(payload + UPVALUE_SLOT).ok()?;
            return value(reader, &reader.read_vec(slot, NODE_KEY).ok()?);
        }
        _ => LuaValue::Other,
    })
}

fn key_tag(key: &[u8]) -> Option<u32> {
    slot_tag(key).map(|tag| tag & KEY_TAG_MASK)
}

fn key_is<R: MemoryReader + ?Sized>(reader: &R, key: &[u8], name: &str) -> bool {
    key_tag(key) == Some(u32::from(STRING_TAG))
        && read_u64_le(key, 0)
            .and_then(|object| read_text(reader, object))
            .as_deref()
            == Some(name)
}

impl LuaTable {
    fn header<R: MemoryReader + ?Sized>(self, reader: &R) -> Option<[u8; TABLE_HEADER]> {
        let mut header = [0u8; TABLE_HEADER];
        reader.read_exact(self.0, &mut header).ok()?;
        (u32::from(header[0]) == TABLE_TAG).then_some(header)
    }

    fn nodes<R: MemoryReader + ?Sized>(self, reader: &R) -> Option<Vec<u8>> {
        let header = self.header(reader)?;
        let bits = header[TABLE_NODE_BITS];
        if bits > MAX_NODE_BITS {
            return None;
        }
        reader
            .read_vec(read_u64_le(&header, TABLE_NODES)?, NODE << bits)
            .ok()
    }

    fn find<R: MemoryReader + ?Sized>(
        self,
        reader: &R,
        is_key: impl Fn(&[u8]) -> bool,
    ) -> Option<LuaValue> {
        let nodes = self.nodes(reader)?;
        let node = nodes
            .as_chunks::<NODE>()
            .0
            .iter()
            .find(|node| is_key(&node[NODE_KEY..]))?;
        value(reader, &node[..NODE_KEY])
    }

    pub fn field<R: MemoryReader + ?Sized>(self, reader: &R, name: &str) -> Option<LuaValue> {
        self.find(reader, |key| key_is(reader, key, name))
    }

    fn shared<R: MemoryReader + ?Sized>(self, reader: &R) -> Option<LuaTable> {
        self.find(reader, |key| {
            key_tag(key) == Some(SHARED_KEY_TAG) && read_u32_le(key, 0) == Some(SHARED_KEY)
        })?
        .table()
    }

    pub fn values<R: MemoryReader + ?Sized>(self, reader: &R) -> Vec<LuaValue> {
        self.nodes(reader)
            .unwrap_or_default()
            .as_chunks::<NODE>()
            .0
            .iter()
            .filter(|node| key_tag(&node[NODE_KEY..]).is_some_and(|tag| tag != 0))
            .filter_map(|node| value(reader, &node[..NODE_KEY]))
            .collect()
    }

    pub fn locals<R: MemoryReader + ?Sized>(self, reader: &R) -> Vec<LuaTable> {
        let mut tables: Vec<LuaTable> = self
            .values(reader)
            .into_iter()
            .filter_map(LuaValue::function)
            .flat_map(|function| function.upvalues(reader))
            .filter_map(LuaValue::table)
            .collect();
        tables.sort_unstable_by_key(|table| table.0);
        tables.dedup();
        tables
    }

    pub fn items<R: MemoryReader + ?Sized>(self, reader: &R) -> Vec<LuaValue> {
        let Some(header) = self.header(reader) else {
            return Vec::new();
        };
        let slots = read_u32_le(&header, TABLE_ARRAY_SIZE)
            .filter(|size| *size <= MAX_ARRAY)
            .and_then(|size| usize::try_from(size).ok())
            .zip(read_u64_le(&header, TABLE_ARRAY))
            .and_then(|(size, array)| reader.read_vec(array, size * NODE_KEY).ok())
            .unwrap_or_default();
        slots
            .as_chunks::<NODE_KEY>()
            .0
            .iter()
            .filter_map(|slot| value(reader, slot))
            .collect()
    }
}

impl LuaFunction {
    pub fn upvalues<R: MemoryReader + ?Sized>(self, reader: &R) -> Vec<LuaValue> {
        let mut header = [0u8; CLOSURE_HEADER];
        if reader.read_exact(self.0, &mut header).is_err()
            || u32::from(header[0]) != FUNCTION_TAG
            || header[CLOSURE_NATIVE] != 0
        {
            return Vec::new();
        }
        reader
            .read_vec(
                self.0 + CLOSURE_SLOTS,
                usize::from(header[CLOSURE_UPVALUES]) * NODE_KEY,
            )
            .unwrap_or_default()
            .as_chunks::<NODE_KEY>()
            .0
            .iter()
            .filter_map(|slot| value(reader, slot))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaState {
    thread: u64,
    globals_field: u64,
    globals: LuaTable,
    registry: LuaTable,
}

impl LuaState {
    pub fn locate<R: MemoryReader + ?Sized>(reader: &R) -> Result<Option<Self>> {
        let regions = reader.regions()?;
        let image = reader.module(GAME_PROCESS)?;
        let heap: Vec<&Region> = regions
            .iter()
            .filter(|region| region.is_anonymous_heap())
            .collect();
        for region in image_data(&regions, &image) {
            let data = reader.read_vec(region.range.start, to_usize(region.size())?)?;
            let found = words(&data)
                .filter(|(_, thread)| in_heap(&heap, *thread))
                .find_map(|(_, thread)| Self::at(reader, &heap, thread));
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }

    fn at<R: MemoryReader + ?Sized>(reader: &R, heap: &[&Region], thread: u64) -> Option<Self> {
        let fields = reader.read_vec(thread, THREAD_FIELDS).ok()?;
        if fields[0] != THREAD_TAG {
            return None;
        }
        let (globals_field, globals) = words(&fields)
            .filter(|(_, table)| in_heap(heap, *table))
            .map(|(offset, table)| (offset, LuaTable(table)))
            .find(|(_, table)| table.shared(reader).is_some())?;
        let registry = words(&fields)
            .filter(|(_, state)| in_heap(heap, *state))
            .find_map(|(_, state)| Self::registry(reader, state, globals))?;
        Some(Self {
            thread,
            globals_field: to_u64(globals_field).ok()?,
            globals,
            registry,
        })
    }

    fn registry<R: MemoryReader + ?Sized>(
        reader: &R,
        global_state: u64,
        globals: LuaTable,
    ) -> Option<LuaTable> {
        let fields = reader.read_vec(global_state, GLOBAL_STATE_FIELDS).ok()?;
        let slot = |offset: usize| fields.get(offset..offset + NODE_KEY);
        let table = |slot: &[u8]| {
            (slot_tag(slot) == Some(TABLE_TAG))
                .then(|| read_u64_le(slot, 0).map(LuaTable))
                .flatten()
        };
        (0..fields.len())
            .step_by(POINTER)
            .filter(|offset| slot(*offset).and_then(table) == Some(globals))
            .filter_map(|offset| slot(offset + NODE_KEY).and_then(table))
            .find(|registry| !registry.items(reader).is_empty())
    }

    pub fn holds<R: MemoryReader + ?Sized>(&self, reader: &R) -> bool {
        let mut tag = [0u8; 1];
        reader.read_exact(self.thread, &mut tag).is_ok()
            && tag[0] == THREAD_TAG
            && reader.read_u64(self.thread + self.globals_field).ok() == Some(self.globals.0)
    }

    pub fn shared<R: MemoryReader + ?Sized>(&self, reader: &R) -> Option<LuaTable> {
        self.globals.shared(reader)
    }

    pub fn script<R: MemoryReader + ?Sized>(&self, reader: &R, owns: &str) -> Option<LuaTable> {
        self.scripts(reader)
            .into_iter()
            .find(|script| script.field(reader, owns).is_some())
    }

    pub fn scripts<R: MemoryReader + ?Sized>(&self, reader: &R) -> Vec<LuaTable> {
        self.registry
            .items(reader)
            .into_iter()
            .filter_map(LuaValue::table)
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::fake::FakeReader;

    pub(crate) const IMAGE: u64 = 0x10_0000;
    pub(crate) const HEAP: u64 = 0x1000;
    const THREAD: usize = 0x100;
    const GLOBAL_STATE: usize = 0x200;
    const GLOBALS: usize = 0x300;
    const REGISTRY: usize = 0x400;
    const SHARED: usize = 0x500;
    pub(crate) const FREE: usize = 0x600;

    pub(crate) type Field = (u64, u32, u64, u32);

    pub(crate) struct Heap {
        pub(crate) bytes: Vec<u8>,
        pub(crate) shared: Vec<Field>,
        next: usize,
    }

    fn address(offset: usize) -> u64 {
        HEAP + u64::try_from(offset).unwrap()
    }

    impl Heap {
        fn put(&mut self, offset: usize, bytes: &[u8]) {
            self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
        }

        fn put_u64(&mut self, offset: usize, word: u64) {
            self.put(offset, &word.to_le_bytes());
        }

        fn put_slot(&mut self, offset: usize, payload: u64, tag: u32) {
            self.put_u64(offset, payload);
            self.put(offset + SLOT_TAG, &tag.to_le_bytes());
        }

        fn take(&mut self, len: usize) -> usize {
            let at = self.next;
            self.next += len.next_multiple_of(16);
            at
        }

        pub(crate) fn text(&mut self, text: &str) -> u64 {
            let at = self.take(STRING_HEADER + text.len());
            self.bytes[at] = STRING_TAG;
            let len = u32::try_from(text.len()).unwrap();
            self.put(at + STRING_HEADER - 4, &len.to_le_bytes());
            self.put(at + STRING_HEADER, text.as_bytes());
            address(at)
        }

        fn table_at(&mut self, at: usize, fields: &[Field], items: &[(u64, u32)]) {
            let bits = fields.len().next_power_of_two().trailing_zeros();
            let nodes = self.take(NODE << bits);
            let array = self.take(16 * items.len().max(1));
            self.bytes[at] = u8::try_from(TABLE_TAG).unwrap();
            self.bytes[at + TABLE_NODE_BITS] = u8::try_from(bits).unwrap();
            let size = u32::try_from(items.len()).unwrap();
            self.put(at + TABLE_ARRAY_SIZE, &size.to_le_bytes());
            self.put_u64(at + TABLE_ARRAY, address(array));
            self.put_u64(at + TABLE_NODES, address(nodes));
            for (index, (key, key_tag, payload, tag)) in fields.iter().enumerate() {
                self.put_slot(nodes + index * NODE, *payload, *tag);
                self.put_slot(nodes + index * NODE + NODE_KEY, *key, *key_tag);
            }
            for (index, (payload, tag)) in items.iter().enumerate() {
                self.put_slot(array + index * 16, *payload, *tag);
            }
        }

        pub(crate) fn table(&mut self, fields: &[Field]) -> u64 {
            let at = self.take(TABLE_HEADER);
            self.table_at(at, fields, &[]);
            address(at)
        }

        pub(crate) fn list(&mut self, items: &[u64]) -> u64 {
            let at = self.take(TABLE_HEADER);
            let items: Vec<(u64, u32)> = items.iter().map(|item| (*item, TABLE_TAG)).collect();
            self.table_at(at, &[], &items);
            address(at)
        }

        pub(crate) fn function_field(&mut self, name: &str, upvalues: &[u64]) -> Field {
            let at = self.take(0x20 + upvalues.len() * NODE_KEY);
            self.bytes[at] = u8::try_from(FUNCTION_TAG).unwrap();
            self.bytes[at + CLOSURE_UPVALUES] = u8::try_from(upvalues.len()).unwrap();
            for (index, table) in upvalues.iter().enumerate() {
                let cell = self.take(0x20);
                self.put_u64(cell + 8, address(cell + 0x10));
                self.put_slot(cell + 0x10, *table, TABLE_TAG);
                self.put_slot(at + 0x20 + index * NODE_KEY, address(cell), UPVALUE_TAG);
            }
            (
                self.text(name),
                u32::from(STRING_TAG),
                address(at),
                FUNCTION_TAG,
            )
        }

        pub(crate) fn text_field(&mut self, name: &str, text: &str) -> Field {
            let tag = u32::from(STRING_TAG);
            (self.text(name), tag, self.text(text), tag)
        }

        pub(crate) fn table_field(&mut self, name: &str, table: u64) -> Field {
            (self.text(name), u32::from(STRING_TAG), table, TABLE_TAG)
        }

        pub(crate) fn with_scripts(scripts: impl FnOnce(&mut Self) -> Vec<u64>) -> FakeReader {
            let mut heap = Self {
                bytes: vec![0u8; 0x4000],
                shared: Vec::new(),
                next: FREE,
            };
            let scripts: Vec<(u64, u32)> = scripts(&mut heap)
                .into_iter()
                .map(|script| (script, TABLE_TAG))
                .collect();
            heap.bytes[THREAD] = THREAD_TAG;
            heap.put_u64(THREAD + 0x18, address(GLOBAL_STATE));
            heap.put_u64(THREAD + 0x58, address(GLOBALS));
            heap.put_slot(GLOBAL_STATE + 0x90, address(GLOBALS), TABLE_TAG);
            heap.put_slot(GLOBAL_STATE + 0xa0, address(REGISTRY), TABLE_TAG);
            let shared = (
                u64::from(SHARED_KEY),
                SHARED_KEY_TAG,
                address(SHARED),
                TABLE_TAG,
            );
            heap.table_at(GLOBALS, &[shared], &[]);
            heap.table_at(REGISTRY, &[], &scripts);
            let sortie = heap.text_field("CachedSortieId", "6aac0afe0bb39ae78a6884e0");
            let shared: Vec<Field> = heap.shared.drain(..).chain([sortie]).collect();
            heap.table_at(SHARED, &shared, &[]);
            let mut image = vec![0u8; 0x100];
            image[0x40..0x48].copy_from_slice(&address(THREAD).to_le_bytes());
            FakeReader::with_image((IMAGE, image), vec![(HEAP, heap.bytes)])
        }
    }

    #[test]
    fn state_through_image_pointer() {
        let reader = Heap::with_scripts(|heap| vec![heap.table(&[])]);
        let state = LuaState::locate(&reader).unwrap().unwrap();
        assert!(state.holds(&reader));
        let shared = state.shared(&reader).unwrap();
        assert_eq!(
            shared.field(&reader, "CachedSortieId"),
            Some(LuaValue::Text("6aac0afe0bb39ae78a6884e0".to_owned()))
        );
        assert_eq!(shared.field(&reader, "Missing"), None);
    }

    #[test]
    fn registry_scripts() {
        let reader = Heap::with_scripts(|heap| {
            let field = heap.text_field("mName", "Relay");
            vec![heap.table(&[field])]
        });
        let state = LuaState::locate(&reader).unwrap().unwrap();
        let scripts = state.scripts(&reader);
        assert_eq!(scripts.len(), 1);
        assert_eq!(
            scripts[0].field(&reader, "mName").and_then(LuaValue::text),
            Some("Relay".to_owned())
        );
    }

    #[test]
    fn missing_image_pointer() {
        let reader =
            FakeReader::with_image((IMAGE, vec![0u8; 0x100]), vec![(HEAP, vec![0u8; 0x100])]);
        assert_eq!(LuaState::locate(&reader).unwrap(), None);
    }
}
