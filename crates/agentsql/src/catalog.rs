use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Database catalog containing all metadata
pub struct DatabaseCatalog {
    tables: RwLock<HashMap<String, CatalogTableSchema>>,
    views: RwLock<HashMap<String, String>>,
}

impl DatabaseCatalog {
    pub fn new() -> Self {
        Self {
            tables: RwLock::new(HashMap::new()),
            views: RwLock::new(HashMap::new()),
        }
    }

    pub fn read(&self) -> Result<&DatabaseCatalog, String> {
        Ok(self)
    }

    pub fn tables(&self) -> Option<Vec<(String, CatalogTableSchema)>> {
        self.tables.read().ok().map(|h| {
            h.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
    }

    pub fn views(&self) -> Option<HashMap<String, String>> {
        self.views.read().ok()
    }

    pub fn add_table(&mut self, name: String, table: CatalogTableSchema) {
        let mut tables = self.tables.write().ok();
        if let Some(tables) = tables {
            tables.insert(name, table);
        }
    }

    pub fn add_view(&mut self, name: String, definition: String) {
        let mut views = self.views.write().ok();
        if let Some(views) = views {
            views.insert(name, definition);
        }
    }

    pub fn remove_table(&mut self, name: &str) -> Option<CatalogTableSchema> {
        let mut tables = self.tables.write().ok();
        if let Some(tables) = tables {
            tables.remove(name)
        } else {
            None
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&CatalogTableSchema> {
        self.tables.read().ok().and_then(|h| h.get(name))
    }

    pub fn clear(&mut self) {
        let mut tables = self.tables.write().ok();
        if let Some(tables) = tables {
            tables.clear();
        }
        let mut views = self.views.write().ok();
        if let Some(views) = views {
            views.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.tables.read().ok().map(|h| h.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for DatabaseCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DatabaseCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tables_count = self.tables.read().ok().map(|h| h.len()).unwrap_or(0);
        let views_count = self.views.read().ok().map(|h| h.len()).unwrap_or(0);
        f.debug_struct("DatabaseCatalog")
            .field("tables_count", &tables_count)
            .field("views_count", &views_count)
            .finish()
    }
}

/// Column metadata
pub struct CatalogColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
    pub max_length: Option<usize>,
    pub precision: Option<usize>,
    pub scale: Option<usize>,
    pub foreign_key: Option<ForeignKeyRef>,
}

impl CatalogColumn {
    pub fn new(
        name: String,
        data_type: String,
        nullable: bool,
        is_primary_key: bool,
        is_unique: bool,
        default_value: Option<String>,
        max_length: Option<usize>,
        precision: Option<usize>,
        scale: Option<usize>,
        foreign_key: Option<ForeignKeyRef>,
    ) -> Self {
        Self {
            name,
            data_type,
            nullable,
            is_primary_key,
            is_unique,
            default_value,
            max_length,
            precision,
            scale,
            foreign_key,
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn data_type(&self) -> &str { &self.data_type }
    pub fn nullable(&self) -> bool { self.nullable }
    pub fn is_primary_key(&self) -> bool { self.is_primary_key }
    pub fn is_unique(&self) -> bool { self.is_unique }
    pub fn default_value(&self) -> Option<&str> { self.default_value.as_deref() }
    pub fn max_length(&self) -> Option<usize> { self.max_length }
    pub fn precision(&self) -> Option<usize> { self.precision }
    pub fn scale(&self) -> Option<usize> { self.scale }
    pub fn foreign_key(&self) -> Option<&ForeignKeyRef> { &self.foreign_key }
}

/// Index metadata
pub struct CatalogIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: String,
}

impl CatalogIndex {
    pub fn new(
        name: String,
        columns: Vec<String>,
        unique: bool,
        index_type: String,
    ) -> Self {
        Self {
            name,
            columns,
            unique,
            index_type,
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn columns(&self) -> &[String] { &self.columns }
    pub fn unique(&self) -> bool { self.unique }
    pub fn index_type(&self) -> &str { &self.index_type }
}

/// Foreign key reference
pub struct ForeignKeyRef {
    pub table: String,
    pub column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

impl ForeignKeyRef {
    pub fn new(
        table: String,
        column: String,
        on_delete: Option<String>,
        on_update: Option<String>,
    ) -> Self {
        Self {
            table,
            column,
            on_delete,
            on_update,
        }
    }

    pub fn table(&self) -> &str { &self.table }
    pub fn column(&self) -> &str { &self.column }
    pub fn on_delete(&self) -> Option<&str> { self.on_delete.as_deref() }
    pub fn on_update(&self) -> Option<&str> { self.on_update.as_deref() }
}

/// Table schema
pub struct CatalogTableSchema {
    pub name: String,
    pub columns: Vec<CatalogColumn>,
    pub indexes: Vec<CatalogIndex>,
    pub comment: Option<String>,
}

impl CatalogTableSchema {
    pub fn new(
        name: String,
        columns: Vec<CatalogColumn>,
        indexes: Vec<CatalogIndex>,
        comment: Option<String>,
    ) -> Self {
        Self {
            name,
            columns,
            indexes,
            comment,
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn columns(&self) -> &[CatalogColumn] { &self.columns }
    pub fn indexes(&self) -> &[CatalogIndex] { &self.indexes }
    pub fn comment(&self) -> Option<&str> { self.comment.as_deref() }
}
