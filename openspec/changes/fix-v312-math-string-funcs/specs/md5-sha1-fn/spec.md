## ADDED Requirements

### Requirement: MD5 哈希函数

MD5(string) 返回输入字符串的 MD5 十六进制哈希值（32 字符）。

#### Scenario: MD5 基本用法
- **WHEN** `SELECT MD5('abc');`
- **THEN** 返回 `900150983cd24fb0d6963f7d28e17f72`

#### Scenario: MD5 空字符串
- **WHEN** `SELECT MD5('');`
- **THEN** 返回 `d41d8cd98f00b204e9800998ecf8427e`

#### Scenario: MD5 NULL 输入
- **WHEN** `SELECT MD5(NULL);`
- **THEN** 返回 `NULL`

### Requirement: SHA1 哈希函数

SHA1(string) 返回输入字符串的 SHA-1 十六进制哈希值（40 字符）。

#### Scenario: SHA1 基本用法
- **WHEN** `SELECT SHA1('abc');`
- **THEN** 返回 `a9993e364706816aba3e25717850c26c9cd0d89d`

#### Scenario: SHA1 NULL 输入
- **WHEN** `SELECT SHA1(NULL);`
- **THEN** 返回 `NULL`
