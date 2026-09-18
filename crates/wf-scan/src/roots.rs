use wf_mem::{Module, Region};

const POINTER: usize = 8;

pub fn image_data<'a>(regions: &'a [Region], image: &Module) -> impl Iterator<Item = &'a Region> {
    let image = image.base..image.base + image.size;
    regions.iter().filter(move |region| {
        region.readable && region.writable && image.contains(&region.range.start)
    })
}

pub fn in_heap(heap: &[&Region], address: u64) -> bool {
    address.is_multiple_of(8) && heap.iter().any(|region| region.range.contains(&address))
}

pub fn words(bytes: &[u8]) -> impl Iterator<Item = (usize, u64)> {
    bytes
        .as_chunks::<POINTER>()
        .0
        .iter()
        .enumerate()
        .map(|(index, word)| (index * POINTER, u64::from_le_bytes(*word)))
}
