use crate::sdk_types::AkaveError;

pub trait Chunkable {
    fn next_chunk(
        &mut self,
        chunk_size: usize,
    ) -> impl std::future::Future<Output = Option<Result<Box<[u8]>, AkaveError>>> + Send;

    fn data_size(&mut self) -> usize;
}

impl Chunkable for Vec<u8> {
    async fn next_chunk(&mut self, chunk_size: usize) -> Option<Result<Box<[u8]>, AkaveError>> {
        if self.is_empty() {
            None
        } else {
            let chunk = self.drain(..chunk_size).collect::<Vec<u8>>(); // Take chunk_size items from the front
            Some(Ok(chunk.into_boxed_slice()))
        }
    }

    fn data_size(&mut self) -> usize {
        self.len()
    }
}
