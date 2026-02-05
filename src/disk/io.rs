use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

use rsext4::error::{BlockDevError, BlockDevResult};
use rsext4::BlockDevice;

pub struct PartitionBlockDev {
    file: File,
    offset: u64,
    total_blocks: u64,
    block_size: u32,
    opened: bool,
}

impl PartitionBlockDev {
    pub fn new(file: File, offset: u64, total_blocks: u64, block_size: u32) -> Self {
        Self {
            file,
            offset,
            total_blocks,
            block_size,
            opened: true,
        }
    }

    fn check_range(&self, block_id: u32, count: u32) -> BlockDevResult<()> {
        let end = block_id as u64 + count as u64;
        if end > self.total_blocks {
            return Err(BlockDevError::BlockOutOfRange {
                block_id,
                max_blocks: self.total_blocks,
            });
        }
        Ok(())
    }
}

impl BlockDevice for PartitionBlockDev {
    fn write(&mut self, buffer: &[u8], block_id: u32, count: u32) -> BlockDevResult<()> {
        if !self.opened {
            return Err(BlockDevError::DeviceClosed);
        }
        self.check_range(block_id, count)?;

        let block_size = self.block_size as usize;
        let required = block_size * count as usize;
        if buffer.len() < required {
            return Err(BlockDevError::BufferTooSmall {
                provided: buffer.len(),
                required,
            });
        }

        let offset = self.offset + block_id as u64 * self.block_size as u64;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| BlockDevError::IoError)?;
        self.file
            .write_all(&buffer[..required])
            .map_err(|_| BlockDevError::WriteError)?;
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8], block_id: u32, count: u32) -> BlockDevResult<()> {
        if !self.opened {
            return Err(BlockDevError::DeviceClosed);
        }
        self.check_range(block_id, count)?;

        let block_size = self.block_size as usize;
        let required = block_size * count as usize;
        if buffer.len() < required {
            return Err(BlockDevError::BufferTooSmall {
                provided: buffer.len(),
                required,
            });
        }

        let offset = self.offset + block_id as u64 * self.block_size as u64;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| BlockDevError::IoError)?;
        self.file
            .read_exact(&mut buffer[..required])
            .map_err(|_| BlockDevError::ReadError)?;
        Ok(())
    }

    fn open(&mut self) -> BlockDevResult<()> {
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) -> BlockDevResult<()> {
        self.opened = false;
        Ok(())
    }

    fn total_blocks(&self) -> u64 {
        self.total_blocks
    }

    fn block_size(&self) -> u32 {
        self.block_size
    }

    fn flush(&mut self) -> BlockDevResult<()> {
        self.file.sync_all().map_err(|_| BlockDevError::IoError)
    }

    fn is_open(&self) -> bool {
        self.opened
    }
}

pub struct PartitionIo {
    file: File,
    start: u64,
    len: u64,
    pos: u64,
}

impl PartitionIo {
    pub fn new(file: File, start: u64, len: u64) -> Self {
        Self {
            file,
            start,
            len,
            pos: 0,
        }
    }

    fn clamp_pos(&self, pos: i128) -> io::Result<u64> {
        if pos < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"));
        }
        let pos = pos as u64;
        if pos > self.len {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek out of range"));
        }
        Ok(pos)
    }
}

impl Read for PartitionIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let remain = self.len - self.pos;
        let to_read = remain.min(buf.len() as u64) as usize;
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.read(&mut buf[..to_read])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for PartitionIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "no space"));
        }
        let remain = self.len - self.pos;
        let to_write = remain.min(buf.len() as u64) as usize;
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.write(&buf[..to_write])?;
        self.pos += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for PartitionIo {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(off) => self.clamp_pos(off as i128)?,
            SeekFrom::End(off) => self.clamp_pos(self.len as i128 + off as i128)?,
            SeekFrom::Current(off) => self.clamp_pos(self.pos as i128 + off as i128)?,
        };
        self.pos = new_pos;
        Ok(self.pos)
    }
}
