//! GMP Document Management
//!
//! Defines document tables and management functions for the GMP extension.
//! Documents are versioned, typed, and can have multiple sections and keywords.

use sqlrustgo_storage::{ColumnDefinition, StorageEngine};
use sqlrustgo_types::{SqlResult, Value};

/// Document status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocStatus {
    Draft,
    Active,
    Archived,
    Superseded,
}

impl DocStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocStatus::Draft => "DRAFT",
            DocStatus::Active => "ACTIVE",
            DocStatus::Archived => "ARCHIVED",
            DocStatus::Superseded => "SUPERSEDED",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Some(DocStatus::Draft),
            "ACTIVE" => Some(DocStatus::Active),
            "ARCHIVED" => Some(DocStatus::Archived),
            "SUPERSEDED" => Some(DocStatus::Superseded),
            _ => None,
        }
    }
}

/// Document table row representation
#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub doc_type: String,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub effective_date: i32,
    pub status: DocStatus,
}

impl Document {
    /// Convert a database row to a Document
    pub fn from_row(row: &[Value]) -> Option<Self> {
        let id = match &row[0] {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let title = match &row[1] {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let doc_type = match &row[2] {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let version = match &row[3] {
            Value::Integer(n) => *n as i32,
            _ => return None,
        };
        let created_at = match &row[4] {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let updated_at = match &row[5] {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let effective_date = match &row[6] {
            Value::Integer(n) => *n as i32,
            _ => return None,
        };
        let status = match &row[7] {
            Value::Text(s) => DocStatus::from_str(s)?,
            _ => return None,
        };

        Some(Document {
            id,
            title,
            doc_type,
            version,
            created_at,
            updated_at,
            effective_date,
            status,
        })
    }

    /// Convert a Document to a database row
    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.id),
            Value::Text(self.title.clone()),
            Value::Text(self.doc_type.clone()),
            Value::Integer(self.version as i64),
            Value::Integer(self.created_at),
            Value::Integer(self.updated_at),
            Value::Integer(self.effective_date as i64),
            Value::Text(self.status.as_str().to_string()),
        ]
    }
}

/// Document content section
#[derive(Debug, Clone)]
pub struct DocumentContent {
    pub doc_id: i64,
    pub section: String,
    pub content: String,
}

impl DocumentContent {
    pub fn from_row(row: &[Value]) -> Option<Self> {
        let doc_id = match &row[0] {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let section = match &row[1] {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let content = match &row[2] {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        Some(DocumentContent {
            doc_id,
            section,
            content,
        })
    }

    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.doc_id),
            Value::Text(self.section.clone()),
            Value::Text(self.content.clone()),
        ]
    }
}

/// Document keyword entry
#[derive(Debug, Clone)]
pub struct DocumentKeyword {
    pub doc_id: i64,
    pub keyword: String,
}

impl DocumentKeyword {
    pub fn from_row(row: &[Value]) -> Option<Self> {
        let doc_id = match &row[0] {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let keyword = match &row[1] {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        Some(DocumentKeyword { doc_id, keyword })
    }

    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.doc_id),
            Value::Text(self.keyword.clone()),
        ]
    }
}

/// GMP document tables constant names
pub const TABLE_DOCUMENTS: &str = "gmp_documents";
pub const TABLE_DOCUMENT_CONTENTS: &str = "gmp_document_contents";
pub const TABLE_DOCUMENT_KEYWORDS: &str = "gmp_document_keywords";

/// SQL statements to create GMP tables
pub const CREATE_DOCUMENTS_TABLE: &str = r#"
CREATE TABLE gmp_documents (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    doc_type TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    effective_date INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT'
)
"#;

pub const CREATE_DOCUMENT_CONTENTS_TABLE: &str = r#"
CREATE TABLE gmp_document_contents (
    doc_id INTEGER NOT NULL,
    section TEXT NOT NULL,
    content TEXT NOT NULL,
    PRIMARY KEY (doc_id, section)
)
"#;

pub const CREATE_DOCUMENT_KEYWORDS_TABLE: &str = r#"
CREATE TABLE gmp_document_keywords (
    doc_id INTEGER NOT NULL,
    keyword TEXT NOT NULL,
    PRIMARY KEY (doc_id, keyword)
)
"#;

/// Create all GMP document tables using the storage engine
pub fn create_gmp_tables(storage: &mut dyn StorageEngine) -> SqlResult<()> {
    // Create gmp_documents table
    if !storage.has_table(TABLE_DOCUMENTS) {
        let columns = vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "doc_type".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "version".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "created_at".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "updated_at".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "effective_date".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "status".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_DOCUMENTS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    // Create gmp_document_contents table
    if !storage.has_table(TABLE_DOCUMENT_CONTENTS) {
        let columns = vec![
            ColumnDefinition {
                name: "doc_id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "section".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "content".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_DOCUMENT_CONTENTS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    // Create gmp_document_keywords table
    if !storage.has_table(TABLE_DOCUMENT_KEYWORDS) {
        let columns = vec![
            ColumnDefinition {
                name: "doc_id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "keyword".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_DOCUMENT_KEYWORDS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    // Create gmp_document_versions table
    if !storage.has_table(crate::schema::TABLE_DOCUMENT_VERSIONS) {
        let columns = crate::schema::document_versions_columns();
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: crate::schema::TABLE_DOCUMENT_VERSIONS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    // Create gmp_chunks table
    if !storage.has_table(crate::schema::TABLE_CHUNKS) {
        let columns = crate::schema::chunks_columns();
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: crate::schema::TABLE_CHUNKS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    // Create gmp_relations table
    if !storage.has_table(crate::schema::TABLE_RELATIONS) {
        let columns = crate::schema::relations_columns();
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: crate::schema::TABLE_RELATIONS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }

    Ok(())
}

/// Builder struct for inserting a new document
pub struct NewDocument<'a> {
    pub title: &'a str,
    pub doc_type: &'a str,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub effective_date: i32,
    pub status: DocStatus,
}

/// Insert a document into the storage engine
pub fn insert_document(storage: &mut dyn StorageEngine, doc: NewDocument) -> SqlResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(doc.updated_at);

    // Get the next auto-increment id BEFORE inserting
    let rows = storage.scan(TABLE_DOCUMENTS)?;
    let next_id = rows
        .iter()
        .filter_map(|r| match &r[0] {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;

    let row = vec![
        Value::Integer(next_id),
        Value::Text(doc.title.to_string()),
        Value::Text(doc.doc_type.to_string()),
        Value::Integer(doc.version as i64),
        Value::Integer(doc.created_at.max(now)),
        Value::Integer(doc.updated_at.max(now)),
        Value::Integer(doc.effective_date as i64),
        Value::Text(doc.status.as_str().to_string()),
    ];

    storage.insert(TABLE_DOCUMENTS, vec![row])?;
    Ok(next_id)
}

/// Insert document content section
pub fn insert_document_content(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    section: &str,
    content: &str,
) -> SqlResult<()> {
    let row = vec![
        Value::Integer(doc_id),
        Value::Text(section.to_string()),
        Value::Text(content.to_string()),
    ];
    storage.insert(TABLE_DOCUMENT_CONTENTS, vec![row])?;
    Ok(())
}

/// Insert document keyword
pub fn insert_document_keyword(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    keyword: &str,
) -> SqlResult<()> {
    let row = vec![Value::Integer(doc_id), Value::Text(keyword.to_string())];
    storage.insert(TABLE_DOCUMENT_KEYWORDS, vec![row])?;
    Ok(())
}

/// Query documents by type
pub fn query_by_type(storage: &dyn StorageEngine, doc_type: &str) -> SqlResult<Vec<Document>> {
    let rows = storage.scan(TABLE_DOCUMENTS)?;
    let docs = rows
        .into_iter()
        .filter_map(|row| {
            let doc = Document::from_row(&row)?;
            if doc.doc_type == doc_type {
                Some(doc)
            } else {
                None
            }
        })
        .collect();
    Ok(docs)
}

/// Query documents by status
pub fn query_by_status(
    storage: &dyn StorageEngine,
    status: &DocStatus,
) -> SqlResult<Vec<Document>> {
    let rows = storage.scan(TABLE_DOCUMENTS)?;
    let docs = rows
        .into_iter()
        .filter_map(|row| {
            let doc = Document::from_row(&row)?;
            if doc.status == *status {
                Some(doc)
            } else {
                None
            }
        })
        .collect();
    Ok(docs)
}

/// Query documents by effective date range
pub fn query_by_effective_date(
    storage: &dyn StorageEngine,
    from_date: i32,
    to_date: i32,
) -> SqlResult<Vec<Document>> {
    let rows = storage.scan(TABLE_DOCUMENTS)?;
    let docs = rows
        .into_iter()
        .filter_map(|row| {
            let doc = Document::from_row(&row)?;
            if doc.effective_date >= from_date && doc.effective_date <= to_date {
                Some(doc)
            } else {
                None
            }
        })
        .collect();
    Ok(docs)
}

/// Get all document keywords for a document
pub fn get_keywords(storage: &dyn StorageEngine, doc_id: i64) -> SqlResult<Vec<String>> {
    let rows = storage.scan(TABLE_DOCUMENT_KEYWORDS)?;
    let keywords = rows
        .into_iter()
        .filter_map(|row| {
            let kw = DocumentKeyword::from_row(&row)?;
            if kw.doc_id == doc_id {
                Some(kw.keyword)
            } else {
                None
            }
        })
        .collect();
    Ok(keywords)
}

/// Get all document content sections for a document
pub fn get_content(storage: &dyn StorageEngine, doc_id: i64) -> SqlResult<Vec<DocumentContent>> {
    let rows = storage.scan(TABLE_DOCUMENT_CONTENTS)?;
    let contents = rows
        .into_iter()
        .filter_map(|row| {
            let content = DocumentContent::from_row(&row)?;
            if content.doc_id == doc_id {
                Some(content)
            } else {
                None
            }
        })
        .collect();
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_doc_status_conversion() {
        assert_eq!(DocStatus::from_str("ACTIVE"), Some(DocStatus::Active));
        assert_eq!(DocStatus::from_str("active"), Some(DocStatus::Active));
        assert_eq!(DocStatus::from_str("UNKNOWN"), None);
        assert_eq!(DocStatus::Active.as_str(), "ACTIVE");
    }

    #[test]
    fn test_document_row_roundtrip() {
        let doc = Document {
            id: 42,
            title: "Test Doc".to_string(),
            doc_type: "POLICY".to_string(),
            version: 3,
            created_at: 1700000000,
            updated_at: 1700100000,
            effective_date: 19000,
            status: DocStatus::Active,
        };
        let row = doc.to_row();
        let recovered = Document::from_row(&row).unwrap();
        assert_eq!(recovered.id, doc.id);
        assert_eq!(recovered.title, doc.title);
        assert_eq!(recovered.doc_type, doc.doc_type);
        assert_eq!(recovered.status, doc.status);
    }

    #[test]
    fn test_create_tables() {
        let mut storage = MemoryStorage::new();
        create_gmp_tables(&mut storage).unwrap();
        assert!(storage.has_table(TABLE_DOCUMENTS));
        assert!(storage.has_table(TABLE_DOCUMENT_CONTENTS));
        assert!(storage.has_table(TABLE_DOCUMENT_KEYWORDS));
    }

    #[test]
    fn test_insert_and_query_document() {
        let mut storage = MemoryStorage::new();
        create_gmp_tables(&mut storage).unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let doc_id = insert_document(
            &mut storage,
            NewDocument {
                title: "Employee Handbook",
                doc_type: "HANDBOOK",
                version: 1,
                created_at: now,
                updated_at: now,
                effective_date: 19000,
                status: DocStatus::Active,
            },
        )
        .unwrap();

        insert_document_content(&mut storage, doc_id, "intro", "Welcome to the company").unwrap();
        insert_document_keyword(&mut storage, doc_id, "HR").unwrap();
        insert_document_keyword(&mut storage, doc_id, "onboarding").unwrap();

        let docs = query_by_type(&storage, "HANDBOOK").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Employee Handbook");

        let keywords = get_keywords(&storage, doc_id).unwrap();
        assert_eq!(keywords.len(), 2);
        assert!(keywords.contains(&"HR".to_string()));

        let contents = get_content(&storage, doc_id).unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].section, "intro");
    }
    #[test]
    fn test_doc_status_all_variants() {
        assert_eq!(DocStatus::Draft.as_str(), "DRAFT");
        assert_eq!(DocStatus::Active.as_str(), "ACTIVE");
        assert_eq!(DocStatus::Archived.as_str(), "ARCHIVED");
        assert_eq!(DocStatus::Superseded.as_str(), "SUPERSEDED");
    }

    #[test]
    fn test_doc_status_from_str_all() {
        assert_eq!(DocStatus::from_str("DRAFT"), Some(DocStatus::Draft));
        assert_eq!(DocStatus::from_str("ACTIVE"), Some(DocStatus::Active));
        assert_eq!(DocStatus::from_str("ARCHIVED"), Some(DocStatus::Archived));
        assert_eq!(
            DocStatus::from_str("SUPERSEDED"),
            Some(DocStatus::Superseded)
        );
        assert_eq!(DocStatus::from_str("draft"), Some(DocStatus::Draft)); // case insensitive
        assert_eq!(DocStatus::from_str("INVALID"), None);
    }

    #[test]
    fn test_document_from_row_invalid() {
        // Wrong type for id (Text instead of Integer)
        let row = vec![
            Value::Text("not int".to_string()),
            Value::Text("t".to_string()),
            Value::Text("d".to_string()),
            Value::Integer(1),
            Value::Integer(0),
            Value::Integer(0),
            Value::Integer(0),
            Value::Text("DRAFT".to_string()),
        ];
        assert!(Document::from_row(&row).is_none());
    }

    #[test]
    fn test_document_from_row_invalid_types() {
        // Wrong type for id
        let row = vec![
            Value::Text("not int".to_string()),
            Value::Text("t".to_string()),
            Value::Text("d".to_string()),
            Value::Integer(1),
            Value::Integer(0),
            Value::Integer(0),
            Value::Integer(0),
            Value::Text("DRAFT".to_string()),
        ];
        assert!(Document::from_row(&row).is_none());
    }

    #[test]
    fn test_document_to_row_full() {
        let doc = Document {
            id: 42,
            title: "Test".to_string(),
            doc_type: "SOP".to_string(),
            version: 1,
            created_at: 1000,
            updated_at: 2000,
            effective_date: 2025000,
            status: DocStatus::Active,
        };
        let row = doc.to_row();
        assert_eq!(row.len(), 8);
        assert_eq!(row[0], Value::Integer(42));
        assert_eq!(row[1], Value::Text("Test".to_string()));
        assert_eq!(row[7], Value::Text("ACTIVE".to_string()));
    }

    #[test]
    fn test_document_from_row_roundtrip() {
        let doc = Document {
            id: 1,
            title: "X".to_string(),
            doc_type: "SOP".to_string(),
            version: 2,
            created_at: 100,
            updated_at: 200,
            effective_date: 2025000,
            status: DocStatus::Archived,
        };
        let row = doc.to_row();
        let back = Document::from_row(&row).unwrap();
        assert_eq!(back.id, doc.id);
        assert_eq!(back.title, doc.title);
        assert_eq!(back.status, doc.status);
    }

    #[test]
    fn test_document_content_from_row() {
        let row = vec![
            Value::Integer(1),
            Value::Text("intro".to_string()),
            Value::Text("body".to_string()),
        ];
        let c = DocumentContent::from_row(&row).unwrap();
        assert_eq!(c.doc_id, 1);
        assert_eq!(c.section, "intro");
        assert_eq!(c.content, "body");
    }

    #[test]
    fn test_document_content_to_row() {
        let c = DocumentContent {
            doc_id: 1,
            section: "s".to_string(),
            content: "c".to_string(),
        };
        let r = c.to_row();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], Value::Integer(1));
    }

    #[test]
    fn test_document_keyword_from_row_to_row() {
        let row = vec![Value::Integer(1), Value::Text("k".to_string())];
        let k = DocumentKeyword::from_row(&row).unwrap();
        assert_eq!(k.doc_id, 1);
        assert_eq!(k.keyword, "k");
        let r = k.to_row();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_document_keyword_from_row_invalid() {
        let row = vec![Value::Integer(1), Value::Integer(2)]; // wrong type
        assert!(DocumentKeyword::from_row(&row).is_none());
    }

    #[test]
    fn test_document_content_from_row_invalid() {
        // Wrong type for section (Integer instead of Text)
        let row = vec![
            Value::Integer(1),
            Value::Integer(99),
            Value::Text("c".to_string()),
        ];
        assert!(DocumentContent::from_row(&row).is_none());
    }

    #[test]
    fn test_table_constants() {
        assert_eq!(TABLE_DOCUMENTS, "gmp_documents");
        assert_eq!(TABLE_DOCUMENT_CONTENTS, "gmp_document_contents");
        assert_eq!(TABLE_DOCUMENT_KEYWORDS, "gmp_document_keywords");
    }
}
