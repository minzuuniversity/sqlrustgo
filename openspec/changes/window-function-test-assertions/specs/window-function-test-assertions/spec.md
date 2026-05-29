## ADDED Requirements

### Requirement: Strong test assertions for window function execution tests
The system SHALL strengthen test assertions in window function execution tests to verify actual return values instead of only checking row count.

#### Scenario: LEAD function returns correct subsequent row values
- **WHEN** executing `SELECT LEAD(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain the correct LEAD values: `[Some(150), Some(120), Some(180), None]` for quarters Q1=100, Q2=150, Q3=120, Q4=180

#### Scenario: LAG function returns correct preceding row values
- **WHEN** executing `SELECT LAG(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain the correct LAG values: `[None, Some(100), Some(150), Some(120)]` for quarters Q1=100, Q2=150, Q3=120, Q4=180

#### Scenario: FIRST_VALUE function returns correct first partition value
- **WHEN** executing `SELECT FIRST_VALUE(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain the correct FIRST_VALUE: `[Some(100), Some(100), Some(100), Some(100)]` for all rows

#### Scenario: LAST_VALUE function returns correct last partition value
- **WHEN** executing `SELECT LAST_VALUE(amount) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain the correct LAST_VALUE: `[Some(100), Some(150), Some(120), Some(180)]` reflecting frame semantics

#### Scenario: NTILE function distributes rows into correct buckets
- **WHEN** executing `SELECT NTILE(3) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain bucket assignments: `[1, 1, 2, 3]` for 4 rows divided into 3 buckets

#### Scenario: NTH_VALUE function returns correct nth row value
- **WHEN** executing `SELECT NTH_VALUE(amount, 2) OVER (ORDER BY quarter) FROM sales WHERE region = 'North'`
- **THEN** the result SHALL contain the 2nd value: `[None, Some(150), None, None]` (only row 2 has a defined 2nd value within its frame)

### Requirement: Test data setup with known deterministic values
The system SHALL provide deterministic test data setup that ensures consistent window function results.

#### Scenario: Sales data setup produces predictable window function output
- **WHEN** setting up test data with `setup_sales_data` function
- **THEN** the data SHALL contain known values: North (Q1=100, Q2=150, Q3=120, Q4=180), South (Q1=200, Q2=250, Q3=220, Q4=280)

### Requirement: Value extraction helper for result validation
The system SHALL provide a consistent pattern for extracting and comparing window function return values.

#### Scenario: Extract integer values from result rows
- **WHEN** extracting values from `ExecutorResult` containing `Value::Integer(i64)` or `Value::Null`
- **THEN** the extraction SHALL convert to `Vec<Option<i64>>` where `Value::Null` maps to `None` and `Value::Integer(i)` maps to `Some(i)`

#### Scenario: Extract float values from result rows
- **WHEN** extracting values from `ExecutorResult` containing `Value::Float(f64)` or `Value::Null`
- **THEN** the extraction SHALL convert to `Vec<Option<f64>>` where `Value::Null` maps to `None` and `Value::Float(f)` maps to `Some(f)`
