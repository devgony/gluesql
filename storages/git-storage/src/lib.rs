#![deny(clippy::str_to_string)]

mod store;
mod store_mut;

use {
    gluesql_core::{
        error::{Error, Result},
        store::{
            AlterTable, CustomFunction, CustomFunctionMut, Index, IndexMut, Metadata, Store,
            StoreMut, Transaction,
        },
    },
    gluesql_csv_storage::CsvStorage,
    gluesql_file_storage::FileStorage,
    gluesql_json_storage::JsonStorage,
    std::process::Command,
    strum_macros::Display,
};

pub struct GitStorage {
    pub storage_base: StorageBase,
    pub path: String,
    pub remote: String,
    pub branch: String,
}

pub enum StorageBase {
    File(FileStorage),
    Csv(CsvStorage),
    Json(JsonStorage),
}

#[derive(Clone, Copy, Display)]
#[strum(serialize_all = "lowercase")]
pub enum StorageType {
    File,
    Csv,
    Json,
}

const DEFAULT_REMOTE: &str = "origin";
const DEFAULT_BRANCH: &str = "main";

trait CommandExt {
    fn execute(&mut self) -> Result<(), Error>;
}

impl CommandExt for Command {
    fn execute(&mut self) -> Result<(), Error> {
        let output = self.output().map_storage_err()?;

        if !output.status.success() {
            return Err(Error::StorageMsg(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}

impl GitStorage {
    pub fn init(path: &str, storage_type: StorageType) -> Result<Self> {
        let storage_base = Self::storage_base(path, storage_type)?;
        Command::new("git")
            .current_dir(path)
            .arg("init")
            .execute()?;

        Ok(Self {
            storage_base,
            path: path.to_owned(),
            remote: DEFAULT_REMOTE.to_owned(),
            branch: DEFAULT_BRANCH.to_owned(),
        })
    }

    pub fn open(path: &str, storage_type: StorageType) -> Result<Self> {
        let storage_base = Self::storage_base(path, storage_type)?;

        Ok(Self {
            storage_base,
            path: path.to_owned(),
            remote: DEFAULT_REMOTE.to_owned(),
            branch: DEFAULT_BRANCH.to_owned(),
        })
    }

    fn storage_base(path: &str, storage_type: StorageType) -> Result<StorageBase> {
        use StorageType::*;

        match storage_type {
            File => FileStorage::new(path).map(StorageBase::File),
            Csv => CsvStorage::new(path).map(StorageBase::Csv),
            Json => JsonStorage::new(path).map(StorageBase::Json),
        }
    }

    pub fn set_remote(&mut self, remote: String) {
        self.remote = remote;
    }

    pub fn set_branch(&mut self, branch: String) {
        self.branch = branch;
    }

    pub fn add_and_commit(&self, message: &str) -> Result<()> {
        Command::new("git")
            .current_dir(&self.path)
            .arg("add")
            .arg(".")
            .execute()?;

        Command::new("git")
            .current_dir(&self.path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .execute()
    }

    pub fn dml_commit(&self, dml: Dml, table_name: &str, n: usize) -> Result<()> {
        if n == 0 {
            return Ok(());
        }

        self.add_and_commit(&format!("[GitStorage::{dml}] {table_name} - {n} rows"))
    }

    pub fn pull(&self) -> Result<()> {
        Command::new("git")
            .current_dir(&self.path)
            .arg("pull")
            .arg(&self.remote)
            .arg(&self.branch)
            .execute()
    }

    pub fn push(&self) -> Result<()> {
        Command::new("git")
            .current_dir(&self.path)
            .arg("push")
            .arg(&self.remote)
            .arg(&self.branch)
            .execute()
    }

    fn get_store(&self) -> &dyn Store {
        match &self.storage_base {
            StorageBase::File(storage) => storage,
            StorageBase::Csv(storage) => storage,
            StorageBase::Json(storage) => storage,
        }
    }

    fn get_store_mut(&mut self) -> &mut dyn StoreMut {
        match &mut self.storage_base {
            StorageBase::File(storage) => storage,
            StorageBase::Csv(storage) => storage,
            StorageBase::Json(storage) => storage,
        }
    }
}

#[derive(Display)]
#[strum(serialize_all = "snake_case")]
pub enum Dml {
    AppendData,
    InsertData,
    DeleteData,
}

pub trait ResultExt<T, E: ToString> {
    fn map_storage_err(self) -> Result<T, Error>;
}

impl<T, E: ToString> ResultExt<T, E> for std::result::Result<T, E> {
    fn map_storage_err(self) -> Result<T, Error> {
        self.map_err(|e| e.to_string()).map_err(Error::StorageMsg)
    }
}

impl AlterTable for GitStorage {}
impl Index for GitStorage {}
impl IndexMut for GitStorage {}
impl Transaction for GitStorage {}
impl Metadata for GitStorage {}
impl CustomFunction for GitStorage {}
impl CustomFunctionMut for GitStorage {}
