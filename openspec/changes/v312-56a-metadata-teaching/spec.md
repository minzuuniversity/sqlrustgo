# Spec — V312-56A: Metadata/SHOW/information_schema

## Overview

实现 MySQL 兼容的 information_schema 和 SHOW 命令，整理为可教学的受控子集。

## Specification

### information_schema

#### information_schema.tables

| Column | Type | Description |
|---|---|---|
| TABLE_CATALOG | varchar(256) | Catalog name |
| TABLE_SCHEMA | varchar(64) | Schema name |
| TABLE_NAME | varchar(64) | Table name |
| TABLE_TYPE | varchar(64) | BASE TABLE / VIEW |
| ENGINE | varchar(64) | Storage engine (if applicable) |

#### information_schema.columns

| Column | Type | Description |
|---|---|---|
| TABLE_CATALOG | varchar(256) | Catalog name |
| TABLE_SCHEMA | varchar(64) | Schema name |
| TABLE_NAME | varchar(64) | Table name |
| COLUMN_NAME | varchar(64) | Column name |
| DATA_TYPE | varchar(64) | Data type |
| IS_NULLABLE | varchar(3) | YES / NO |
| COLUMN_KEY | varchar(3) | PRI / UNI / MUL |
| COLUMN_DEFAULT | text | Default value |

#### information_schema.indexes

| Column | Type | Description |
|---|---|---|
| TABLE_SCHEMA | varchar(64) | Schema name |
| TABLE_NAME | varchar(64) | Table name |
| INDEX_NAME | varchar(64) | Index name |
| COLUMN_NAME | varchar(64) | Column name |
| NON_UNIQUE | int | 0=unique, 1=non-unique |
| INDEX_TYPE | varchar(64) | BTREE / HASH |

### SHOW Commands

#### SHOW CREATE TABLE

输出与 live schema 一致，包含:
- column type
- NULL
- primary key
- default value

#### SHOW COLUMNS

输出格式与 MySQL 兼容:
- Field, Type, Null, Key, Default, Extra

#### SHOW INDEX

输出:
- Table, Non_unique, Key_name, Seq_in_index, Column_name, Index_type

#### DESCRIBE

等价于 SHOW COLUMNS。

## Boundaries

- information_schema 查询必须 fail-closed (返回明确错误) 而非静默忽略
- LIKE pattern 在 SHOW COLUMNS 中若暂不支持，返回明确错误
