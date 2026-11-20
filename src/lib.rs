//! A synchronous `http` store for the [`zarrs`](https://docs.rs/zarrs/latest/zarrs/index.html) crate.
//!
//! ```rust
//! # use std::sync::Arc;
//! use zarrs_storage::AsyncReadableStorage;
//! use zarrs_http_async::AsyncHTTPStore;
//!
//! let http_store: AsyncReadableStorage = Arc::new(AsyncHttpStore::new("http://...")?);
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
use std::str::FromStr;

use futures::StreamExt;
use itertools::multiunzip;
use reqwest::{
    Client, StatusCode, Url,
    header::{CONTENT_LENGTH, HeaderValue, RANGE},
};
use thiserror::Error;
use zarrs_storage::{
    AsyncMaybeBytesIterator, AsyncReadableStorageTraits, MaybeBytes, StorageError, StoreKey,
    byte_range::ByteRangeIterator,
};

#[derive(Debug, Error)]
pub enum AsyncHttpStoreCreateError {
    #[error("base URL {0} is not valid")]
    InvalidBaseURL(String),
}

pub struct AsyncHttpStore {
    base_url: Url,
    client: Client,
}

impl AsyncHttpStore {
    /// Create a new asynchronous HTTP store at a given `base_url`.
    ///
    /// # Errors
    ///
    /// Returns a Returns a [`AsyncHttpStoreCreateError`] if `base_url` is not a valid URL.
    pub fn new(base_url: &str) -> Result<Self, AsyncHttpStoreCreateError> {
        let base_url = Url::from_str(base_url)
            .map_err(|_err| AsyncHttpStoreCreateError::InvalidBaseURL(base_url.into()))?;

        let client = Client::new();

        Ok(Self { base_url, client })
    }

    /// Maps a [`StoreKey`] to a HTTP [`Url`].
    ///
    /// # Errors
    ///
    /// Returns a [`url::ParseError`] if the URL is invalid.
    pub fn key_to_url(&self, key: &StoreKey) -> Result<Url, url::ParseError> {
        let mut url = self.base_url.clone();

        if !key.as_str().is_empty() {
            url.path_segments_mut()
                .map_err(|_| url::ParseError::RelativeUrlWithoutBase)?
                .pop_if_empty()
                .extend(key.as_str().trim_prefix('/').split('/'));
        }

        Ok(url)
    }
}

fn handle_reqwest_error(err: reqwest::Error) -> StorageError {
    StorageError::Other(err.to_string())
}

fn handle_url_error(err: url::ParseError) -> StorageError {
    StorageError::Other(err.to_string())
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(
    target_arch = "wasm32",
    async_trait::async_trait(?Send)
)]
impl AsyncReadableStorageTraits for AsyncHttpStore {
    async fn get(&self, key: &StoreKey) -> Result<MaybeBytes, StorageError> {
        let url = self.key_to_url(key).map_err(handle_url_error)?;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        match response.status() {
            StatusCode::OK => Ok(Some(response.bytes().await.map_err(handle_reqwest_error)?)),
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(StorageError::from(format!(
                "http unexpected status code: {}",
                response.status(),
            ))),
        }
    }

    async fn get_partial_many<'a>(
        &'a self,
        key: &StoreKey,
        byte_ranges: ByteRangeIterator<'a>,
    ) -> Result<AsyncMaybeBytesIterator<'a>, StorageError> {
        let url = self.key_to_url(key).map_err(handle_url_error)?;
        let Some(size) = self.size_key(key).await? else {
            return Ok(None);
        };
        let (bytes_strs, bytes_lengths, bytes_start_end): (
            Vec<String>,
            Vec<usize>,
            Vec<(usize, usize)>,
        ) = multiunzip(byte_ranges.map(|byte_range| {
            (
                format!("{}-{}", byte_range.start(size), byte_range.end(size) - 1),
                usize::try_from(byte_range.length(size)).unwrap(),
                (
                    usize::try_from(byte_range.start(size)).unwrap(),
                    usize::try_from(byte_range.end(size)).unwrap(),
                ),
            )
        }));
        let bytes_strs = bytes_strs.join(", ");

        let range = HeaderValue::from_str(&format!("bytes={bytes_strs}")).unwrap();
        let response = self
            .client
            .get(url)
            .header(RANGE, range)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        match response.status() {
            StatusCode::NOT_FOUND => Err(StorageError::from(
                "the http server returned a NOT FOUND status for the byte range request, but returned a non zero size for CONTENT_LENGTH",
            )),
            StatusCode::PARTIAL_CONTENT => {
                // TODO: Gracefully handle a response from the server which does not include all requested by ranges
                let mut bytes = response.bytes().await.map_err(handle_reqwest_error)?;
                if bytes.len() as u64 == bytes_lengths.iter().sum::<usize>() as u64 {
                    let bytes = bytes_lengths
                        .into_iter()
                        .map(move |length| Ok(bytes.split_to(length)));
                    Ok(Some(futures::stream::iter(bytes).boxed()))
                } else {
                    Err(StorageError::from(
                        "http partial content response did not include all requested byte ranges",
                    ))
                }
            }
            StatusCode::OK => {
                // Received all bytes
                let bytes = response.bytes().await.map_err(handle_reqwest_error)?;
                let bytes = bytes_start_end
                    .into_iter()
                    .map(move |(start, end)| Ok(bytes.slice(start..end)));
                Ok(Some(futures::stream::iter(bytes).boxed()))
            }
            _ => Err(StorageError::from(format!(
                "the http server responded with status {} for the byte range request",
                response.status()
            ))),
        }
    }

    async fn size_key(&self, key: &StoreKey) -> Result<Option<u64>, StorageError> {
        let url = self.key_to_url(key).map_err(handle_url_error)?;
        let response = self
            .client
            .head(url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        match response.status() {
            StatusCode::OK => {
                let length = response
                    .headers()
                    .get(CONTENT_LENGTH)
                    .and_then(|header_value| header_value.to_str().ok())
                    .and_then(|header_str| u64::from_str(header_str).ok())
                    .ok_or_else(|| StorageError::from("content length response is invalid"))?;
                Ok(Some(length))
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(StorageError::from(format!(
                "http size_key has status code {}",
                response.status()
            ))),
        }
    }

    fn supports_get_partial(&self) -> bool {
        true
    }
}
