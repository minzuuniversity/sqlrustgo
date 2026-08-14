//! GMP Schema Definitions for v3.12
//!
//! This module defines all GMP table schemas as SQL constants and column definitions.
//! All CREATE TABLE statements use IF NOT EXISTS for idempotent schema creation.
//!
//! ## Tables
//!
//! - `gmp_document_versions`: Version history per document with source hash
//! - `gmp_chunks`: Chunk-level content with content hash
//! - `gmp_relations`: Typed edges between documents/chunks

use sqlrustgo_storage::ColumnDefinition;

// ============================================================================
// Table Names
// ============================================================================

/// GMP document versions table
pub const TABLE_DOCUMENT_VERSIONS: &str = "gmp_document_versions";
/// GMP chunks table
pub const TABLE_CHUNKS: &str = "gmp_chunks";
/// GMP relations table
pub const TABLE_RELATIONS: &str = "gmp_relations";

// ============================================================================
// Document Versions
// ============================================================================

/// SQL to create the document versions table
pub const CREATE_DOCUMENT_VERSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS gmp_document_versions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL,
    version_number  INTEGER NOT NULL DEFAULT 1,
    source_hash     TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    change_desc     TEXT,
    UNIQUE(doc_id, version_number)
)
"#;

/// Column definitions for gmp_document_versions
pub fn document_versions_columns() -> Vec<ColumnDefinition> {
    vec![
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
            name: "doc_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "version_number".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "source_hash".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "content_hash".to_string(),
            data_type: "TEXT".to_string(),
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
            name: "change_desc".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
    ]
}

// ============================================================================
// Chunks
// ============================================================================

/// SQL to create the chunks table
pub const CREATE_CHUNKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS gmp_chunks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL,
    version_number  INTEGER NOT NULL,
    chunk_index     INTEGER NOT NULL,
    section_name    TEXT,
    content_hash    TEXT NOT NULL,
    content_text    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    UNIQUE(doc_id, version_number, chunk_index)
)
"#;

/// Column definitions for gmp_chunks
pub fn chunks_columns() -> Vec<ColumnDefinition> {
    vec![
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
            name: "doc_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "version_number".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "chunk_index".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "section_name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "content_hash".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "content_text".to_string(),
            data_type: "TEXT".to_string(),
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
    ]
}

// ============================================================================
// Relations
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash)]
pub enum RelationType {
    Sop,
    Clause,
    Capa,
    Deviation,
    Role,
    Equipment,
    AuditFinding,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::Sop => "SOP",
            RelationType::Clause => "CLAUSE",
            RelationType::Capa => "CAPA",
            RelationType::Deviation => "DEVIATION",
            RelationType::Role => "ROLE",
            RelationType::Equipment => "EQUIPMENT",
            RelationType::AuditFinding => "AUDIT_FINDING",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SOP" => Some(RelationType::Sop),
            "CLAUSE" => Some(RelationType::Clause),
            "CAPA" => Some(RelationType::Capa),
            "DEVIATION" => Some(RelationType::Deviation),
            "ROLE" => Some(RelationType::Role),
            "EQUIPMENT" => Some(RelationType::Equipment),
            "AUDIT_FINDING" => Some(RelationType::AuditFinding),
            _ => None,
        }
    }

    pub fn all_str() -> Vec<&'static str> {
        vec![
            "SOP",
            "CLAUSE",
            "CAPA",
            "DEVIATION",
            "ROLE",
            "EQUIPMENT",
            "AUDIT_FINDING",
        ]
    }
}

/// SQL to create the relations table
pub const CREATE_RELATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS gmp_relations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_doc_id   INTEGER,
    source_chunk_id INTEGER,
    relation_type   TEXT NOT NULL,
    target_doc_id   INTEGER,
    target_chunk_id INTEGER,
    properties      TEXT,
    created_at      INTEGER NOT NULL,
    UNIQUE(source_doc_id, source_chunk_id, relation_type, target_doc_id, target_chunk_id)
)
"#;

/// Column definitions for gmp_relations
pub fn relations_columns() -> Vec<ColumnDefinition> {
    vec![
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
            name: "source_doc_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "source_chunk_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "relation_type".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "target_doc_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "target_chunk_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
        },
        ColumnDefinition {
            name: "properties".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
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
    ]
}
