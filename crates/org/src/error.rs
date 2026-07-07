use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrgError {
    #[error("org not found: {0}")]
    NotFound(String),

    #[error("user {0} is not a member of org {1}")]
    NotMember(String, String),

    #[error("forbidden: requires {0}")]
    Forbidden(String),

    #[error("slug already taken: {0}")]
    SlugTaken(String),

    #[error("cannot remove the last owner of org {0}")]
    LastOwner(String),
}

// Note: The plan's original `impl From<OrgError> for StorageError` is omitted
// because `llm_gateway_storage` does not define a `StorageError` enum — its
// `Storage` trait returns `Box<dyn std::error::Error + Send + Sync>` directly.
// Callers that need to surface an `OrgError` through a storage-bound code path
// should box it via `Box::new(err) as Box<dyn std::error::Error + Send + Sync>`
// (see Task 7 / Task 5).
