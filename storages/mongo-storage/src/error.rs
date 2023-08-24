use {gluesql_core::error::Error, thiserror::Error};

pub trait ResultExt<T, E: ToString> {
    fn map_storage_err(self) -> Result<T, Error>;
}

impl<T, E: ToString> ResultExt<T, E> for std::result::Result<T, E> {
    fn map_storage_err(self) -> Result<T, Error> {
        self.map_err(|e| e.to_string()).map_err(Error::StorageMsg)
    }
}

pub trait OptionExt<T, E: ToString> {
    fn map_storage_err(self, error: E) -> Result<T, Error>;
}

impl<T, E: ToString> OptionExt<T, E> for std::option::Option<T> {
    fn map_storage_err(self, error: E) -> Result<T, Error> {
        self.ok_or_else(|| error.to_string())
            .map_err(Error::StorageMsg)
    }
}

#[derive(Error, Debug)]
pub enum MongoStorageError {
    #[error("table does not exist")]
    TableDoesNotExist,

    #[error("index does not have option")]
    MissingIndexOption,

    #[error("composite index is not supported")]
    CompositIndexNotSupported,

    #[error("invalid document")]
    InvalidDocument,

    #[error("unreachable")]
    Unreachable,

    #[error("unsupported bson type")]
    UnsupportedBsonType,
}
