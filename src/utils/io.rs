use std::io::{self, Read};

pub struct PaddedReader<R: Read> {
    inner: R,
    total: u64,
    read: u64,
    inner_done: bool,
}

impl<R: Read> PaddedReader<R> {
    pub fn new(inner: R, total: u64) -> Self {
        Self {
            inner,
            total,
            read: 0,
            inner_done: false,
        }
    }
}

impl<R: Read> Read for PaddedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.read >= self.total {
            return Ok(0);
        }
        let remaining = (self.total - self.read) as usize;
        let to_fill = buf.len().min(remaining);
        let target = &mut buf[..to_fill];

        if self.inner_done {
            target.iter_mut().for_each(|b| *b = 0);
            self.read += to_fill as u64;
            return Ok(to_fill);
        }

        match self.inner.read(target) {
            Ok(0) => {
                self.inner_done = true;
                target.iter_mut().for_each(|b| *b = 0);
                self.read += to_fill as u64;
                Ok(to_fill)
            }
            Ok(n) => {
                self.read += n as u64;
                Ok(n)
            }
            Err(e) => Err(e),
        }
    }
}

pub fn random_file(size: usize) -> Vec<u8> {
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
    (0..size)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as u8
        })
        .collect()
}

pub fn skip_to_position<R: Read>(mut reader: R, n: u64) -> io::Result<R> {
    let mut buf = [0u8; 4096];
    let mut remaining = n;
    while remaining > 0 {
        let to_read = buf.len().min(remaining as usize);
        let got = reader.read(&mut buf[..to_read])?;
        if got == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "reader exhausted before reaching skip position",
            ));
        }
        remaining -= got as u64;
    }
    Ok(reader)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::io::Read;

    use super::{random_file, skip_to_position, PaddedReader};

    #[test]
    fn test_padded_reader() {
        let data_size = 12 * 1024usize;
        let padded_size = 16 * 1024usize;

        let data: Vec<u8> = (0..data_size).map(|i| (i % 251) as u8).collect();
        let cursor = std::io::Cursor::new(data.clone());
        let mut padded = PaddedReader::new(cursor, padded_size as u64);

        let mut result = Vec::new();
        padded.read_to_end(&mut result).unwrap();

        assert_eq!(result.len(), padded_size);
        assert_eq!(&result[..data_size], data.as_slice());
        assert_eq!(
            &result[data_size..],
            vec![0u8; padded_size - data_size].as_slice()
        );
    }

    #[test]
    fn test_random_file() {
        let size = 4 * 1024;
        let data1 = random_file(size);
        let data2 = random_file(size);

        assert_eq!(data1.len(), size);
        assert_eq!(data1, data2, "random_file must be deterministic");

        let min = *data1.iter().min().unwrap();
        let max = *data1.iter().max().unwrap();
        assert!(
            (max as i32 - min as i32) > 100,
            "random_file should have significant byte value variation"
        );
    }

    #[test]
    fn test_skip_to_position() {
        let data: Vec<u8> = (0u8..=255).cycle().take(1024).collect();

        let cursor = std::io::Cursor::new(data.clone());
        let mut reader = skip_to_position(cursor, 100).unwrap();
        let mut tail = Vec::new();
        reader.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, &data[100..]);

        let cursor2 = std::io::Cursor::new(data.clone());
        let mut reader2 = skip_to_position(cursor2, 0).unwrap();
        let mut all = Vec::new();
        reader2.read_to_end(&mut all).unwrap();
        assert_eq!(all, data);

        let cursor3 = std::io::Cursor::new(vec![1u8, 2, 3]);
        assert!(skip_to_position(cursor3, 10).is_err());
    }
}
