//! Cypher AST.

use crate::types::PropertyValue;

/// A parsed Cypher query.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// Top-level body: a sequence of reading clauses (MATCH / OPTIONAL MATCH / WITH).
    pub body: Vec<Clause>,
    /// Return projection.
    pub return_clause: ReturnClause,
    /// Optional UNION/UNION ALL with more (body, return) pairs.
    pub unions: Vec<UnionArm>,
}

/// One arm of a UNION (or UNION ALL).
#[derive(Debug, Clone, PartialEq)]
pub struct UnionArm {
    /// `true` for UNION ALL, `false` for plain UNION (deduplicated).
    pub all: bool,
    /// Body clauses of this arm.
    pub body: Vec<Clause>,
    /// Return projection of this arm.
    pub return_clause: ReturnClause,
}

/// A single clause in the query body.
#[derive(Debug, Clone, PartialEq)]
pub enum Clause {
    /// `MATCH <pattern>` — inner joins over the bound context.
    Match(Pattern),
    /// `OPTIONAL MATCH <pattern>` — left joins, may produce nulls.
    OptionalMatch(Pattern),
    /// `WITH <projection> [WHERE <expr>]` — materialise intermediate rows.
    With {
        /// Projection expressions.
        projection: Vec<Projection>,
        /// Optional filter applied after materialisation.
        where_expr: Option<Expr>,
    },
    /// `WHERE <expr>` — row filter (top-level body clause).
    Where(Expr),
}

/// RETURN or WITH projection item.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    /// The expression being projected.
    pub expr: Expr,
    /// Optional alias (`AS <name>`).
    pub alias: Option<String>,
}

/// RETURN clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnClause {
    /// Distinct (deduplicate rows)?
    pub distinct: bool,
    /// Projection items.
    pub items: Vec<Projection>,
    /// Optional ORDER BY (not implemented yet — present in AST for forward compat).
    pub order_by: Vec<Projection>,
    /// Optional SKIP.
    pub skip: Option<i64>,
    /// Optional LIMIT.
    pub limit: Option<i64>,
}

/// A graph pattern: a sequence of (node, edge, node, edge, node, …).
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// First node in the chain.
    pub first: NodePattern,
    /// Remaining edges and nodes.
    pub rest: Vec<(EdgePattern, NodePattern)>,
}

/// Node pattern: `(n:Label {prop: value})`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePattern {
    /// Variable name (required in our subset — Cypher permits anonymous, we don't).
    pub var: String,
    /// Labels the node must carry.
    pub labels: Vec<String>,
    /// Inline property map filter (all properties must equal the given values).
    pub properties: Vec<(String, PropertyValue)>,
}

/// Edge pattern: `-[:TYPE {prop: value}]->` or `-[var:TYPE]->` etc.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePattern {
    /// Variable name (optional).
    pub var: Option<String>,
    /// Edge types; empty means "any type".
    pub types: Vec<String>,
    /// Inline property map filter.
    pub properties: Vec<(String, PropertyValue)>,
    /// Direction: `Out`, `In`, or `Either`.
    pub direction: EdgeDirection,
}

/// Edge direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    /// `(a)-[r]->(b)`
    Out,
    /// `(a)<-[r]-(b)` (b->a in storage)
    In,
    /// `(a)-[r]-(b)` — either direction
    Either,
}

/// Expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value.
    Literal(PropertyValue),
    /// Bare identifier (column or bound variable).
    Ident(String),
    /// Property access: `expr.prop`.
    Property {
        /// The expression yielding a node/edge.
        expr: Box<Expr>,
        /// Property key.
        prop: String,
    },
    /// Binary comparison/arithmetic (we only emit comparisons; arithmetic operators
    /// may appear but are not implemented in the executor).
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Logical AND / OR.
    Logical {
        /// Operator.
        op: LogicalOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Logical NOT.
    Not(Box<Expr>),
    /// `IS NULL` / `IS NOT NULL`.
    IsNull {
        /// Inner expression.
        expr: Box<Expr>,
        /// `true` for IS NULL, `false` for IS NOT NULL.
        negated: bool,
    },
    /// `count(expr)` aggregate (only form we currently support; non-aggregating context errors).
    Count(Box<Expr>),
}

/// Binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// Logical operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    /// `AND`
    And,
    /// `OR`
    Or,
}
