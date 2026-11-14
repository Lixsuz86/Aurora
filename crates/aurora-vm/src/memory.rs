//! Simulated Linear Memory for low-level pointer and allocation operations.

use thiserror::Error;

/// Errors occurring during low-level memory operations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum MemoryError {
    #[error("Null pointer dereference: attempted to access address 0x0")]
    NullPointerDereference,
    #[error("Memory access out of bounds: address 0x{address:08x} for size {size} bytes")]
    OutOfBounds { address: usize, size: usize },
    #[error(
        "Unaligned memory access: address 0x{address:08x} is not aligned to {alignment} bytes"
    )]
    Unaligned { address: usize, alignment: usize },
}

/// Simulated Linear Memory with boundary checking and safe emulation of raw pointers.
#[derive(Debug, Clone)]
pub struct LinearMemory {
    bytes: Vec<u8>,
    base_address: usize,
    free_offset: usize,
}

impl Default for LinearMemory {
    fn default() -> Self {
        Self::new(64 * 1024) // Default 64 KB initial memory
    }
}

impl LinearMemory {
    /// Creates a new linear memory space with specified initial capacity.
    /// Base address starts at 0x1000 to cleanly catch null pointer dereferences (0x0).
    pub fn new(initial_capacity: usize) -> Self {
        let capacity = initial_capacity.max(4096);
        Self {
            bytes: vec![0; capacity],
            base_address: 0x1000,
            free_offset: 0,
        }
    }

    /// Allocates `size` bytes aligned to `align` boundary.
    pub fn allocate(&mut self, size: usize, align: usize) -> usize {
        let align = align.max(1);
        let remainder = self.free_offset % align;
        if remainder != 0 {
            self.free_offset += align - remainder;
        }

        let addr = self.base_address + self.free_offset;
        self.free_offset += size;

        // Automatically expand backing memory if needed
        if self.free_offset > self.bytes.len() {
            let new_size = (self.free_offset * 2).max(self.bytes.len() * 2);
            self.bytes.resize(new_size, 0);
        }

        addr
    }

    /// Deallocates memory at `_ptr` (linear bump allocator placeholder).
    pub fn free(&mut self, _ptr: usize) {
        // In this bump allocator design, memory is reused per execution or cleared
    }

    /// Translates raw memory address to internal byte slice offset.
    fn get_offset(&self, addr: usize, size: usize) -> Result<usize, MemoryError> {
        if addr == 0 {
            return Err(MemoryError::NullPointerDereference);
        }

        if addr < self.base_address {
            return Err(MemoryError::OutOfBounds {
                address: addr,
                size,
            });
        }

        let offset = addr - self.base_address;
        if offset.checked_add(size).is_none() || offset + size > self.bytes.len() {
            return Err(MemoryError::OutOfBounds {
                address: addr,
                size,
            });
        }

        Ok(offset)
    }

    /// Reads a single byte (`u8`).
    pub fn read_u8(&self, addr: usize) -> Result<u8, MemoryError> {
        let offset = self.get_offset(addr, 1)?;
        Ok(self.bytes[offset])
    }

    /// Writes a single byte (`u8`).
    pub fn write_u8(&mut self, addr: usize, val: u8) -> Result<(), MemoryError> {
        let offset = self.get_offset(addr, 1)?;
        self.bytes[offset] = val;
        Ok(())
    }

    /// Reads a 64-bit signed integer (`i64`) little-endian.
    pub fn read_i64(&self, addr: usize) -> Result<i64, MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        let chunk: [u8; 8] =
            self.bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| MemoryError::OutOfBounds {
                    address: addr,
                    size: 8,
                })?;
        Ok(i64::from_le_bytes(chunk))
    }

    /// Writes a 64-bit signed integer (`i64`) little-endian.
    pub fn write_i64(&mut self, addr: usize, val: i64) -> Result<(), MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        self.bytes[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    /// Reads a 64-bit unsigned integer (`u64`) little-endian.
    pub fn read_u64(&self, addr: usize) -> Result<u64, MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        let chunk: [u8; 8] =
            self.bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| MemoryError::OutOfBounds {
                    address: addr,
                    size: 8,
                })?;
        Ok(u64::from_le_bytes(chunk))
    }

    /// Writes a 64-bit unsigned integer (`u64`) little-endian.
    pub fn write_u64(&mut self, addr: usize, val: u64) -> Result<(), MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        self.bytes[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    /// Reads a 64-bit float (`f64`) little-endian.
    pub fn read_f64(&self, addr: usize) -> Result<f64, MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        let chunk: [u8; 8] =
            self.bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| MemoryError::OutOfBounds {
                    address: addr,
                    size: 8,
                })?;
        Ok(f64::from_le_bytes(chunk))
    }

    /// Writes a 64-bit float (`f64`) little-endian.
    pub fn write_f64(&mut self, addr: usize, val: f64) -> Result<(), MemoryError> {
        let offset = self.get_offset(addr, 8)?;
        self.bytes[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation_and_read_write() {
        let mut mem = LinearMemory::new(1024);
        let ptr = mem.allocate(8, 8);
        assert_eq!(ptr, 0x1000);

        mem.write_i64(ptr, 1337).unwrap();
        assert_eq!(mem.read_i64(ptr).unwrap(), 1337);

        let ptr2 = mem.allocate(8, 8);
        assert_eq!(ptr2, 0x1008);
        mem.write_f64(ptr2, 3.14159).unwrap();
        assert!((mem.read_f64(ptr2).unwrap() - 3.14159).abs() < 1e-5);
    }

    #[test]
    fn test_null_pointer_dereference() {
        let mut mem = LinearMemory::new(1024);
        assert_eq!(mem.read_i64(0), Err(MemoryError::NullPointerDereference));
        assert_eq!(
            mem.write_i64(0, 42),
            Err(MemoryError::NullPointerDereference)
        );
    }
}
