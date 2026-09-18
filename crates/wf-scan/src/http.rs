use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};

use wf_mem::{GAME_PROCESS, MemoryReader, Region, read_u32_le, read_u64_le};

use crate::Result;
use crate::chunks::{to_u64, to_usize};
use crate::inventory::{InventoryBuffer, MAX_INVENTORY_BODY, accept};
use crate::roots::{image_data, in_heap, words};

const CLIENT_FIELDS: usize = 0xc0;
const REQUESTS: usize = 0x18;
const ENTRY_FIELDS: usize = 0x100;
const POINTER: usize = 8;
const STRING: usize = 16;
const STRING_LENGTH: usize = 8;
const STRING_TAG: usize = 15;
const HEAP_STRING: u8 = 0xff;
const LENGTH_MASK: u32 = 0x0fff_ffff;
const BLOCK_ENTRIES: u64 = 2;
const MAX_BLOCKS: u64 = 1 << 20;
const MAX_QUEUED: u64 = 64;
const QUEUES: usize = 2;
const DOCUMENT_HEAD: &[u8] = b"{\"";
const POLL: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Queue {
    map: u64,
    blocks: u64,
    front: u64,
    count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Client {
    slot: u64,
    queues: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpClients {
    clients: Vec<Client>,
}

fn queue(fields: &[u8], at: usize) -> Option<Queue> {
    let map = read_u64_le(fields, at)?;
    let blocks = read_u64_le(fields, at + POINTER)?;
    let front = read_u64_le(fields, at + 2 * POINTER)?;
    let count = read_u64_le(fields, at + 3 * POINTER)?;
    let slots = blocks.checked_mul(BLOCK_ENTRIES)?;
    (map != 0
        && blocks.is_power_of_two()
        && blocks <= MAX_BLOCKS
        && front <= slots
        && count <= slots)
        .then_some(Queue {
            map,
            blocks,
            front,
            count,
        })
}

fn requests<R: MemoryReader + ?Sized>(reader: &R, fields: &[u8], object: u64) -> Option<bool> {
    let head = object.checked_add(to_u64(REQUESTS).ok()?)?;
    let next = read_u64_le(fields, REQUESTS)?;
    let prev = read_u64_le(fields, REQUESTS + POINTER)?;
    if next == head && prev == head {
        return Some(true);
    }
    let back = reader
        .read_u64(next.checked_add(to_u64(POINTER).ok()?)?)
        .ok()?;
    Some(back == head && reader.read_u64(prev).ok()? == head)
}

fn client_at<R: MemoryReader + ?Sized>(
    reader: &R,
    heap: &[&Region],
    object: u64,
) -> Option<Vec<usize>> {
    let fields = reader.read_vec(object, CLIENT_FIELDS).ok()?;
    if fields.len() != CLIENT_FIELDS || !requests(reader, &fields, object)? {
        return None;
    }
    let queues: Vec<usize> = words(&fields)
        .map(|(at, _)| at)
        .filter(|at| queue(&fields, *at).is_some_and(|queue| in_heap(heap, queue.map)))
        .collect();
    (queues.len() >= QUEUES).then_some(queues)
}

fn entry<R: MemoryReader + ?Sized>(reader: &R, queue: &Queue, index: u64) -> Option<u64> {
    let pointer = to_u64(POINTER).ok()?;
    let block = (index / BLOCK_ENTRIES) & (queue.blocks - 1);
    let block = reader
        .read_u64(queue.map.checked_add(block.checked_mul(pointer)?)?)
        .ok()?;
    reader
        .read_u64(block.checked_add(index % BLOCK_ENTRIES * pointer)?)
        .ok()
}

fn strings(fields: &[u8]) -> Vec<(u64, u64)> {
    words(fields)
        .filter_map(|(at, pointer)| {
            let raw = fields.get(at..at + STRING)?;
            let length = u64::from(read_u32_le(raw, STRING_LENGTH)? & LENGTH_MASK);
            (raw[STRING_TAG] == HEAP_STRING
                && pointer != 0
                && length >= to_u64(DOCUMENT_HEAD.len()).ok()?
                && length <= to_u64(MAX_INVENTORY_BODY).ok()?)
            .then_some((pointer, length))
        })
        .collect()
}

fn document<R: MemoryReader + ?Sized>(reader: &R, pointer: u64, length: u64) -> Option<Vec<u8>> {
    if reader.read_vec(pointer, DOCUMENT_HEAD.len()).ok()? != DOCUMENT_HEAD {
        return None;
    }
    let mut body = vec![0u8; to_usize(length).ok()?];
    reader.read_exact(pointer, &mut body).ok()?;
    Some(body)
}

impl HttpClients {
    pub fn locate<R: MemoryReader + ?Sized>(reader: &R) -> Result<Option<Self>> {
        let regions = reader.regions()?;
        let image = reader.module(GAME_PROCESS)?;
        let heap: Vec<&Region> = regions
            .iter()
            .filter(|region| region.is_anonymous_heap())
            .collect();
        let mut clients = Vec::new();
        for region in image_data(&regions, &image) {
            let data = reader.read_vec(region.range.start, to_usize(region.size())?)?;
            for (at, object) in words(&data) {
                if !in_heap(&heap, object) {
                    continue;
                }
                if let Some(queues) = client_at(reader, &heap, object) {
                    clients.push(Client {
                        slot: region.range.start + to_u64(at)?,
                        queues,
                    });
                }
            }
        }
        Ok((!clients.is_empty()).then_some(Self { clients }))
    }

    pub fn slots(&self) -> Vec<u64> {
        self.clients.iter().map(|client| client.slot).collect()
    }

    fn bodies<R: MemoryReader + ?Sized>(&self, reader: &R) -> Vec<(u64, u64)> {
        let mut bodies = Vec::new();
        for client in &self.clients {
            let Ok(object) = reader.read_u64(client.slot) else {
                continue;
            };
            let Ok(fields) = reader.read_vec(object, CLIENT_FIELDS) else {
                continue;
            };
            for at in &client.queues {
                let Some(queue) = queue(&fields, *at) else {
                    continue;
                };
                for index in queue.front..queue.front.saturating_add(queue.count.min(MAX_QUEUED)) {
                    let Some(entry) = entry(reader, &queue, index) else {
                        continue;
                    };
                    let Ok(fields) = reader.read_vec(entry, ENTRY_FIELDS) else {
                        continue;
                    };
                    bodies.extend(strings(&fields));
                }
            }
        }
        bodies
    }

    pub fn await_inventory<R: MemoryReader + ?Sized>(
        &self,
        reader: &R,
        window: Duration,
    ) -> Option<InventoryBuffer> {
        let deadline = Instant::now() + window;
        let mut seen = HashSet::new();
        loop {
            for (pointer, len) in self.bodies(reader) {
                if !seen.insert((pointer, len)) {
                    continue;
                }
                let Some(body) = document(reader, pointer, len) else {
                    continue;
                };
                tracing::debug!(
                    ptr = format!("{pointer:#x}"),
                    len,
                    "HTTP response body seen in the queue"
                );
                if let Some(buffer) = accept(pointer, &body) {
                    return Some(buffer);
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(POLL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeReader;
    use crate::inventory::REQUIRED_KEYS;

    const IMAGE: u64 = 0x1000_0000;
    const IMAGE_SLOT: usize = 0x40;
    const HEAP: u64 = 0x2000_0000;
    const CLIENT: usize = 0x0000;
    const PENDING_MAP: usize = 0x0200;
    const PENDING_BLOCK: usize = 0x0240;
    const DONE_MAP: usize = 0x0300;
    const DONE_BLOCKS: [usize; 2] = [0x0340, 0x0380];
    const ENTRIES: [usize; 2] = [0x0400, 0x0600];
    const BODIES: [usize; 2] = [0x1000, 0x8000];
    const BODY_FIELD: usize = 0x50;
    const PENDING: usize = 0x58;
    const DONE: usize = 0x98;
    const OID: &str = "6a9f2bde000000000000c104";

    struct Heap(Vec<u8>);

    impl Heap {
        fn put(&mut self, at: usize, bytes: &[u8]) {
            self.0[at..at + bytes.len()].copy_from_slice(bytes);
        }

        fn word(&mut self, at: usize, value: u64) {
            self.put(at, &value.to_le_bytes());
        }

        fn address(&mut self, at: usize, offset: usize) {
            self.word(at, HEAP + u64::try_from(offset).unwrap());
        }

        fn string(&mut self, at: usize, offset: usize, length: usize) {
            self.address(at, offset);
            let field = u32::try_from(length).unwrap() | !LENGTH_MASK;
            self.put(at + STRING_LENGTH, &field.to_le_bytes());
            self.0[at + STRING_TAG] = HEAP_STRING;
        }

        fn queue(&mut self, at: usize, map: usize, blocks: u64, front: u64, count: u64) {
            self.address(at, map);
            self.word(at + POINTER, blocks);
            self.word(at + 2 * POINTER, front);
            self.word(at + 3 * POINTER, count);
        }

        fn entry(&mut self, index: usize, body: &str) {
            let at = ENTRIES[index];
            let text = BODIES[index];
            self.put(text, body.as_bytes());
            self.string(at + BODY_FIELD, text, body.len());
        }
    }

    fn inventory_json(oid: &str) -> String {
        let keys = REQUIRED_KEYS
            .iter()
            .filter(|key| **key != "LastInventorySync")
            .map(|key| format!("\"{key}\":[]"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{keys},\"LastInventorySync\":{{\"$oid\":\"{oid}\"}}}}")
    }

    fn heap(count: u64) -> Heap {
        let mut heap = Heap(vec![0u8; 0x40_0000]);
        heap.address(CLIENT + REQUESTS, CLIENT + REQUESTS);
        heap.address(CLIENT + REQUESTS + POINTER, CLIENT + REQUESTS);
        heap.queue(CLIENT + PENDING, PENDING_MAP, 1, 0, 0);
        heap.address(PENDING_MAP, PENDING_BLOCK);
        heap.queue(CLIENT + DONE, DONE_MAP, 2, 1, count);
        heap.address(DONE_MAP, DONE_BLOCKS[0]);
        heap.address(DONE_MAP + POINTER, DONE_BLOCKS[1]);
        heap.address(DONE_BLOCKS[0] + POINTER, ENTRIES[0]);
        heap.address(DONE_BLOCKS[1], ENTRIES[1]);
        heap.entry(0, &inventory_json(OID));
        heap.entry(1, "{\"WorldSeed\":\"x\",\"Events\":[]}");
        heap
    }

    fn image() -> Vec<u8> {
        let mut image = vec![0u8; 0x100];
        image[IMAGE_SLOT..IMAGE_SLOT + POINTER].copy_from_slice(&HEAP.to_le_bytes());
        image
    }

    fn reader(heap: Heap) -> FakeReader {
        FakeReader::with_image((IMAGE, image()), vec![(HEAP, heap.0)])
    }

    fn clients(heap: Heap) -> Option<HttpClients> {
        HttpClients::locate(&reader(heap)).unwrap()
    }

    #[test]
    fn client_in_data_section() {
        let clients = clients(heap(2)).unwrap();
        assert_eq!(
            clients.slots(),
            vec![IMAGE + u64::try_from(IMAGE_SLOT).unwrap()]
        );
    }

    #[test]
    fn completed_queue_inventory() {
        let reader = reader(heap(2));
        let clients = HttpClients::locate(&reader).unwrap().unwrap();
        let buffer = clients.await_inventory(&reader, Duration::ZERO).unwrap();
        assert_eq!(buffer.last_sync, OID);
        assert_eq!(buffer.addr, HEAP + u64::try_from(BODIES[0]).unwrap());
        assert_eq!(buffer.len, inventory_json(OID).len());
        assert_eq!(buffer.json, inventory_json(OID));
    }

    #[test]
    fn queue_count_bound() {
        let one = reader(heap(1));
        assert_eq!(
            HttpClients::locate(&one)
                .unwrap()
                .unwrap()
                .bodies(&one)
                .len(),
            1
        );
        let empty = reader(heap(0));
        let clients = HttpClients::locate(&empty).unwrap().unwrap();
        assert!(clients.bodies(&empty).is_empty());
        assert_eq!(
            clients.await_inventory(&empty, Duration::from_millis(3)),
            None
        );
    }

    #[test]
    fn non_inventory_response() {
        let mut heap = heap(2);
        heap.entry(0, "{\"WorldSeed\":\"x\",\"Events\":[]}");
        let reader = reader(heap);
        let clients = HttpClients::locate(&reader).unwrap().unwrap();
        assert_eq!(clients.await_inventory(&reader, Duration::ZERO), None);
    }

    #[test]
    fn unreadable_body() {
        let mut heap = heap(2);
        heap.string(ENTRIES[0] + BODY_FIELD, BODIES[0], 4 << 20);
        let reader = reader(heap);
        let clients = HttpClients::locate(&reader).unwrap().unwrap();
        assert_eq!(clients.await_inventory(&reader, Duration::ZERO), None);
    }

    #[test]
    fn bare_list_head() {
        let mut heap = heap(2);
        heap.word(CLIENT + PENDING + POINTER, 3);
        heap.word(CLIENT + DONE + POINTER, 0);
        assert_eq!(clients(heap), None);
    }

    #[test]
    fn missing_list_head() {
        let mut heap = heap(2);
        heap.address(CLIENT + REQUESTS, CLIENT + PENDING);
        assert_eq!(clients(heap), None);
    }

    #[test]
    fn single_node_request_list() {
        let mut heap = heap(2);
        let node = 0x2000;
        heap.address(CLIENT + REQUESTS, node);
        heap.address(CLIENT + REQUESTS + POINTER, node);
        heap.address(node, CLIENT + REQUESTS);
        heap.address(node + POINTER, CLIENT + REQUESTS);
        assert!(clients(heap).is_some());
    }

    #[test]
    fn unlinked_request_node() {
        let mut heap = heap(2);
        heap.address(CLIENT + REQUESTS, 0x2000);
        heap.address(CLIENT + REQUESTS + POINTER, 0x2000);
        assert_eq!(clients(heap), None);
    }

    #[test]
    fn wrapping_queue_indices() {
        let found = reader(heap(2));
        let clients = HttpClients::locate(&found).unwrap().unwrap();

        let mut rejected = heap(2);
        rejected.queue(CLIENT + DONE, DONE_MAP, 1 << 62, u64::MAX - 1, 2);
        assert!(clients.bodies(&reader(rejected)).is_empty());

        let mut absurd = heap(2);
        absurd.queue(CLIENT + DONE, DONE_MAP, 1 << 62, 1 << 63, 1 << 63);
        assert!(clients.bodies(&reader(absurd)).is_empty());

        let mut unmapped = heap(2);
        unmapped.word(CLIENT + DONE, 0);
        assert!(clients.bodies(&reader(unmapped)).is_empty());
    }
}
