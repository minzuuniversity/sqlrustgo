## ADDED Requirements

### Requirement: Executor MUST evaluate TRUNCATE/TRUNC

`TRUNCATE(x, d)` truncates `x` to `d` decimal places (d defaults 0). Similar to ROUND but truncates toward zero.

#### Scenario: `TRUNCATE(PI(), 3)` returns 3.141

- **WHEN** executor runs `SELECT TRUNCATE(3.14159, 3)`
- **THEN** result MUST be `3.141`

#### Scenario: `TRUNCATE(123.456)` defaults to 0 decimals

- **WHEN** executor runs `SELECT TRUNCATE(123.456)`
- **THEN** result MUST be `123.0`

#### Scenario: `TRUNCATE(123.456, -1)` truncates to tens

- **WHEN** executor runs `SELECT TRUNCATE(123.456, -1)`
- **THEN** result MUST be `120.0`

#### Scenario: NULL input returns NULL

- **WHEN** executor runs `SELECT TRUNCATE(NULL, 2)`
- **THEN** result MUST be NULL

### Requirement: Executor MUST evaluate HEX

`HEX(x)` converts an integer or blob to uppercase hexadecimal.

#### Scenario: `HEX(255)` returns `'FF'`

- **WHEN** executor runs `SELECT HEX(255)`
- **THEN** result MUST be `'FF'`

#### Scenario: `HEX(16)` returns `'10'`

- **WHEN** executor runs `SELECT HEX(16)`
- **THEN** result MUST be `'10'`

#### Scenario: NULL returns NULL

- **WHEN** executor runs `SELECT HEX(NULL)`
- **THEN** result MUST be NULL

### Requirement: Executor MUST evaluate MD5

`MD5(string)` returns 32-char lowercase hex MD5 hash.

#### Scenario: `MD5('hello')` returns known hash

- **WHEN** executor runs `SELECT MD5('hello')`
- **THEN** result MUST be `'5d41402abc4b2a76b9719d911017c592'`
