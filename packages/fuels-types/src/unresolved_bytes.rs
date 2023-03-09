use crate::constants::WORD_SIZE;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Data {
    // Write the enclosed data immediately.
    Inline(Vec<u8>),
    // The enclosed data should be written somewhere else and only a pointer
    // should be left behind to point to it.
    Dynamic(Vec<Data>),
}

// To get the final encoded bytes, we need to know the address at which these
// bytes are going to be loaded at. Once the address is given to `resolve`
// normal bytes can be retrieved.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct UnresolvedBytes {
    data: Vec<Data>,
}

impl UnresolvedBytes {
    pub fn new(data: Vec<Data>) -> Self {
        Self { data }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Default::default()
    }

    /// Uses the `start_addr` to resolve any pointers contained within. Once
    /// they are resolved the raw bytes are returned.
    ///
    /// # Arguments
    ///
    /// * `start_addr`: The address at which the encoded bytes are to be loaded
    ///                 in.
    pub fn resolve(&self, start_addr: u64) -> Vec<u8> {
        Self::resolve_data(&self.data, start_addr)
    }

    fn resolve_data(data: &[Data], start_addr: u64) -> Vec<u8> {
        // We must find a place for the dynamic data where it will not bother
        // anyone. Best place for it is immediately after all the inline/normal
        // data is encoded.

        let start_of_dynamic_data = start_addr + Self::amount_of_inline_bytes(data);

        let mut inline_data: Vec<u8> = vec![];
        let mut dynamic_data: Vec<u8> = vec![];
        for chunk in data {
            match chunk {
                Data::Inline(bytes) => inline_data.extend(bytes),
                Data::Dynamic(chunk_of_dynamic_data) => {
                    let ptr_to_next_free_location: u64 =
                        start_of_dynamic_data + dynamic_data.len() as u64;

                    // If this is a vector, its `ptr` will now be encoded, the
                    // `cap` and `len` parts should follow as two Data::Inline
                    // chunks.
                    inline_data.extend(ptr_to_next_free_location.to_be_bytes().to_vec());

                    // The dynamic data could have had more dynamic data inside
                    // of it -- think of a Vec<Vec<...>>. Hence Data::Dynamic
                    // doesn't contain bytes but rather more `Data`.
                    let resolved_dynamic_data =
                        Self::resolve_data(chunk_of_dynamic_data, ptr_to_next_free_location);

                    dynamic_data.extend(resolved_dynamic_data)
                }
            }
        }

        let mut data = inline_data;
        data.extend(dynamic_data);
        data
    }

    fn amount_of_inline_bytes(data: &[Data]) -> u64 {
        data.iter()
            .map(|chunk| match chunk {
                Data::Inline(bytes) => bytes.len(),
                Data::Dynamic(_) => {
                    // Only the ptr is encoded inline
                    WORD_SIZE
                }
            } as u64)
            .sum()
    }
}
