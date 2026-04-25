use anyhow::{Context, Result};
use wasmi::{Memory, Store};

use crate::abi::HostState;

pub fn read_memory(memory: &Memory, store: &Store<HostState>, ptr: u32, len: u32) -> Vec<u8> {
    let data = memory.data(store);
    let start = ptr as usize;
    let end = start + len as usize;
    if end <= data.len() {
        data[start..end].to_vec()
    } else {
        Vec::new()
    }
}

pub fn read_string(memory: &Memory, store: &Store<HostState>, ptr: u32, len: u32) -> String {
    let bytes = read_memory(memory, store, ptr, len);
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn write_memory(
    memory: &Memory,
    store: &mut Store<HostState>,
    ptr: u32,
    data: &[u8],
) -> Result<()> {
    let start = ptr as usize;
    let end = start + data.len();
    let memory_data = memory.data_mut(store);
    if end <= memory_data.len() {
        memory_data[start..end].copy_from_slice(data);
        Ok(())
    } else {
        anyhow::bail!("write out of bounds: {}..{} > {}", start, end, memory_data.len())
    }
}

pub fn get_memory(instance: &wasmi::Instance, store: &Store<HostState>) -> Result<Memory> {
    instance
        .get_memory(store, "memory")
        .context("WASM module does not export 'memory'")
}
