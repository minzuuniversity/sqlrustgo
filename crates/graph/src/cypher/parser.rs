//! Cypher parser.
//!
//! Hand-written recursive-descent. The grammar we accept (M4 subset):
//!
//! ```text
//! query        := single_query ( union_op single_query )*
//! single_query := clause* return_clause
//! union_op     := UNION ALL?     (UNION alone = UNION DISTINCT)
//! clause       := MATCH pattern | OPTIONAL MATCH pattern
//!               | WITH projection (',' projection)* (WHERE expr)?
//!               | WHERE expr
//! return_clause:= RETURN [DISTINCT] projection (',' projection)*
//!               (ORDER BY projection (',' projection)*)? (SKIP Integer)? (LIMIT Integer)?
//! pattern      := node (edge node)*
//! node         := '(' var=Ident (':' Ident)* ('{' prop=Ident ':' expr (',' ...)* '}')? ')'
//! edge         := '-[' var=Ident? (':' Ident)* ('{' ... '}')? ']->'
//!               |  '<-[' var=Ident? ... ']-'
//!               |  '-[' var=Ident? ... ']-'
//! projection   := expr (AS Ident)?
//! expr         := or_expr
//! or_expr      := and_expr (OR and_expr)*
//! and_expr     := not_expr (AND not_expr)*
//! not_expr     := NOT? primary
//! primary      := literal | Ident ('.' Ident)* | '(' expr ')'
//!               | count '(' expr ')'
//! literal      := Integer | Float | String | NULL | TRUE | FALSE
//! ```
//!
//! Anonymous node patterns (`()`) are not accepted — every node must bind a
//! variable. This is a deliberate simplification; if needed later, treat
//! missing var as a fresh anonymous binding.

use super::ast::{
    BinaryOp, Clause, EdgeDirection, EdgePattern, Expr, LogicalOp, NodePattern, Pattern,
    Projection, Query, ReturnClause, UnionArm,
};
use super::lexer::{Token, TokenKind};
use crate::types::{GraphError, GraphResult, PropertyValue};

/// Parse a Cypher source string into a [`Query`].
pub fn parse(source: &str) -> GraphResult<Query> {
    let tokens = super::lexer::lex(source)?;
    let mut p = Parser { tokens, pos: 0 };
    let query = p.parse_query()?;
    // After a successful parse, the next token must be EOF.
    if !matches!(p.tokens[p.pos].kind, TokenKind::Ident(ref s) if s.is_empty()) {
        // The EOF sentinel is `Ident("")`. If we still have non-EOF tokens,
        // there was trailing garbage. Find the first such.
        let tok = &p.tokens[p.pos];
        return Err(GraphError::CypherParse {
            line: tok.line,
            col: tok.col,
            message: format!("unexpected trailing token: {:?}", tok.kind),
        });
    }
    Ok(query)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn at_eof(&self) -> bool {
        matches!(self.tokens[self.pos].kind, TokenKind::Ident(ref s) if s.is_empty())
    }

    fn expect(&mut self, kind: TokenKind) -> GraphResult<Token> {
        if std::mem::discriminant(&self.tokens[self.pos].kind) == std::mem::discriminant(&kind) {
            Ok(self.bump())
        } else {
            let t = &self.tokens[self.pos];
            Err(GraphError::CypherParse {
                line: t.line,
                col: t.col,
                message: format!("expected {:?}, got {:?}", kind, t.kind),
            })
        }
    }

    fn parse_query(&mut self) -> GraphResult<Query> {
        let mut body = Vec::new();
        // Body clauses until we hit RETURN (or WITH followed by something other than
        // MATCH/OPTIONAL MATCH, but we only need a single WITH support).
        loop {
            match self.peek() {
                TokenKind::Match | TokenKind::Optional => {
                    let c = self.parse_clause()?;
                    body.push(c);
                }
                TokenKind::Where => {
                    let c = self.parse_clause()?;
                    body.push(c);
                }
                TokenKind::With => {
                    let c = self.parse_with_clause()?;
                    body.push(c);
                }
                _ => break,
            }
        }
        let return_clause = self.parse_return_clause()?;
        let mut unions = Vec::new();
        while matches!(self.peek(), TokenKind::Union) {
            self.bump(); // UNION
            let all = matches!(self.peek(), TokenKind::All);
            if all {
                self.bump();
            }
            let mut arm_body = Vec::new();
            loop {
                match self.peek() {
                    TokenKind::Match | TokenKind::Optional | TokenKind::Where => {
                        arm_body.push(self.parse_clause()?);
                    }
                    TokenKind::With => arm_body.push(self.parse_with_clause()?),
                    _ => break,
                }
            }
            let arm_return = self.parse_return_clause()?;
            unions.push(UnionArm {
                all,
                body: arm_body,
                return_clause: arm_return,
            });
        }
        Ok(Query {
            body,
            return_clause,
            unions,
        })
    }

    fn parse_clause(&mut self) -> GraphResult<Clause> {
        match self.peek() {
            TokenKind::Match => {
                self.bump();
                let pattern = self.parse_pattern()?;
                Ok(Clause::Match(pattern))
            }
            TokenKind::Optional => {
                self.bump();
                self.expect(TokenKind::Match)?;
                let pattern = self.parse_pattern()?;
                Ok(Clause::OptionalMatch(pattern))
            }
            TokenKind::Where => {
                self.bump();
                let expr = self.parse_expr()?;
                Ok(Clause::Where(expr))
            }
            other => {
                let t = &self.tokens[self.pos];
                Err(GraphError::CypherParse {
                    line: t.line,
                    col: t.col,
                    message: format!("expected MATCH/OPTIONAL/WHERE, got {other:?}"),
                })
            }
        }
    }

    fn parse_with_clause(&mut self) -> GraphResult<Clause> {
        self.expect(TokenKind::With)?;
        let projection = self.parse_projection_list()?;
        let where_expr = if matches!(self.peek(), TokenKind::Where) {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Clause::With {
            projection,
            where_expr,
        })
    }

    fn parse_projection_list(&mut self) -> GraphResult<Vec<Projection>> {
        let mut items = Vec::new();
        items.push(self.parse_projection()?);
        while matches!(self.peek(), TokenKind::Comma) {
            self.bump();
            items.push(self.parse_projection()?);
        }
        Ok(items)
    }

    fn parse_projection(&mut self) -> GraphResult<Projection> {
        let expr = self.parse_expr()?;
        let alias = if matches!(self.peek(), TokenKind::As) {
            self.bump();
            let tok = self.bump();
            match tok.kind {
                TokenKind::Ident(s) => Some(s),
                other => {
                    return Err(GraphError::CypherParse {
                        line: tok.line,
                        col: tok.col,
                        message: format!("expected identifier after AS, got {other:?}"),
                    })
                }
            }
        } else {
            None
        };
        Ok(Projection { expr, alias })
    }

    fn parse_return_clause(&mut self) -> GraphResult<ReturnClause> {
        self.expect(TokenKind::Return)?;
        let distinct = if matches!(self.peek(), TokenKind::Distinct) {
            self.bump();
            true
        } else {
            false
        };
        let items = self.parse_projection_list()?;
        // ORDER BY (skip implementation; parse and ignore).
        let order_by = if matches!(self.peek(), TokenKind::Order) {
            self.bump();
            self.expect(TokenKind::By)?;
            self.parse_projection_list()?
        } else {
            Vec::new()
        };
        let skip = if matches!(self.peek(), TokenKind::Skip) {
            self.bump();
            let tok = self.bump();
            match tok.kind {
                TokenKind::Integer(n) => Some(n),
                other => {
                    return Err(GraphError::CypherParse {
                        line: tok.line,
                        col: tok.col,
                        message: format!("expected integer after SKIP, got {other:?}"),
                    })
                }
            }
        } else {
            None
        };
        let limit = if matches!(self.peek(), TokenKind::Limit) {
            self.bump();
            let tok = self.bump();
            match tok.kind {
                TokenKind::Integer(n) => Some(n),
                other => {
                    return Err(GraphError::CypherParse {
                        line: tok.line,
                        col: tok.col,
                        message: format!("expected integer after LIMIT, got {other:?}"),
                    })
                }
            }
        } else {
            None
        };
        Ok(ReturnClause {
            distinct,
            items,
            order_by,
            skip,
            limit,
        })
    }

    fn parse_pattern(&mut self) -> GraphResult<Pattern> {
        let first = self.parse_node_pattern()?;
        let mut rest = Vec::new();
        loop {
            // A `-` token is only the start of an edge pattern if the
            // following token is `[` (i.e. `-[...]`). Otherwise `-` belongs
            // to whatever syntax surrounds us (e.g. unary minus in expressions
            // — though we don't support that yet).
            let is_edge_start = matches!(self.peek(), TokenKind::DashGt | TokenKind::LtDash)
                || (matches!(self.peek(), TokenKind::Dash)
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.kind),
                        Some(TokenKind::LBrack)
                    ));
            if !is_edge_start {
                break;
            }
            let edge = self.parse_edge_pattern()?;
            let node = self.parse_node_pattern()?;
            rest.push((edge, node));
        }
        Ok(Pattern { first, rest })
    }

    fn parse_node_pattern(&mut self) -> GraphResult<NodePattern> {
        self.expect(TokenKind::LParen)?;
        let var_tok = self.bump();
        let var = match var_tok.kind {
            TokenKind::Ident(s) => {
                if s.is_empty() {
                    return Err(GraphError::CypherParse {
                        line: var_tok.line,
                        col: var_tok.col,
                        message: "anonymous node pattern not supported".into(),
                    });
                }
                s
            }
            other => {
                return Err(GraphError::CypherParse {
                    line: var_tok.line,
                    col: var_tok.col,
                    message: format!("expected node variable, got {other:?}"),
                })
            }
        };
        let mut labels = Vec::new();
        while matches!(self.peek(), TokenKind::Colon) {
            self.bump();
            let tok = self.bump();
            match tok.kind {
                TokenKind::Ident(s) => labels.push(s),
                other => {
                    return Err(GraphError::CypherParse {
                        line: tok.line,
                        col: tok.col,
                        message: format!("expected label identifier, got {other:?}"),
                    })
                }
            }
        }
        let properties = if matches!(self.peek(), TokenKind::LBrace) {
            self.parse_inline_properties()?
        } else {
            Vec::new()
        };
        self.expect(TokenKind::RParen)?;
        Ok(NodePattern {
            var,
            labels,
            properties,
        })
    }

    fn parse_edge_pattern(&mut self) -> GraphResult<EdgePattern> {
        // Determine leading direction.
        let leading = match self.peek() {
            TokenKind::DashGt => {
                self.bump();
                EdgeDirection::Out
            }
            TokenKind::LtDash => {
                self.bump();
                EdgeDirection::In
            }
            TokenKind::Dash => {
                self.bump();
                EdgeDirection::Either
            }
            other => {
                let t = &self.tokens[self.pos];
                return Err(GraphError::CypherParse {
                    line: t.line,
                    col: t.col,
                    message: format!("expected '->' / '<-' / '-', got {other:?}"),
                });
            }
        };
        self.expect(TokenKind::LBrack)?;
        let mut var = None;
        let mut types = Vec::new();
        // Optional variable and/or types.
        if matches!(self.peek(), TokenKind::Ident(_)) {
            let tok = self.bump();
            if let TokenKind::Ident(s) = tok.kind {
                if matches!(self.peek(), TokenKind::Colon) {
                    // It's a type label, not a variable.
                    types.push(s);
                } else {
                    var = Some(s);
                    if matches!(self.peek(), TokenKind::Colon) {
                        self.bump();
                        loop {
                            let tok = self.bump();
                            match tok.kind {
                                TokenKind::Ident(s) => types.push(s),
                                other => {
                                    return Err(GraphError::CypherParse {
                                        line: tok.line,
                                        col: tok.col,
                                        message: format!("expected type identifier, got {other:?}"),
                                    })
                                }
                            }
                            if matches!(self.peek(), TokenKind::Pipe) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        } else if matches!(self.peek(), TokenKind::Colon) {
            self.bump();
            loop {
                let tok = self.bump();
                match tok.kind {
                    TokenKind::Ident(s) => types.push(s),
                    other => {
                        return Err(GraphError::CypherParse {
                            line: tok.line,
                            col: tok.col,
                            message: format!("expected type identifier, got {other:?}"),
                        })
                    }
                }
                if matches!(self.peek(), TokenKind::Pipe) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        let properties = if matches!(self.peek(), TokenKind::LBrace) {
            self.parse_inline_properties()?
        } else {
            Vec::new()
        };
        self.expect(TokenKind::RBrack)?;
        // Trailing marker determines the final direction:
        //   `->`   (DashGt) and leading was `-` (Either) -> resolves to Out
        //   `->`   and leading was `->` (Out)            -> stays Out
        //   `<-`   (LtDash) and leading was `-` (Either) -> resolves to In
        //   `<-`   and leading was `<-` (In)             -> stays In
        //   `-`    (Dash) and leading was `-` (Either)   -> stays Either
        //   nothing plain `-[KNOWS]-`                    -> stays Either (rare)
        let direction = match self.peek() {
            TokenKind::DashGt => {
                self.bump();
                match leading {
                    EdgeDirection::In => EdgeDirection::In, // `<-[..]->` is invalid Cypher; we accept In
                    _ => EdgeDirection::Out,
                }
            }
            TokenKind::LtDash => {
                self.bump();
                match leading {
                    EdgeDirection::Out => EdgeDirection::Out, // `->[..]<-` invalid; accept Out
                    _ => EdgeDirection::In,
                }
            }
            TokenKind::Dash => {
                self.bump();
                EdgeDirection::Either
            }
            _ => leading,
        };
        Ok(EdgePattern {
            var,
            types,
            properties,
            direction,
        })
    }

    fn parse_inline_properties(&mut self) -> GraphResult<Vec<(String, PropertyValue)>> {
        self.expect(TokenKind::LBrace)?;
        let mut out = Vec::new();
        if !matches!(self.peek(), TokenKind::RBrace) {
            loop {
                let key_tok = self.bump();
                let key = match key_tok.kind {
                    TokenKind::Ident(s) => s,
                    TokenKind::String(s) => s,
                    other => {
                        return Err(GraphError::CypherParse {
                            line: key_tok.line,
                            col: key_tok.col,
                            message: format!("expected property name, got {other:?}"),
                        })
                    }
                };
                self.expect(TokenKind::Colon)?;
                let value = self.parse_literal()?;
                out.push((key, value));
                if matches!(self.peek(), TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(out)
    }

    fn parse_literal(&mut self) -> GraphResult<PropertyValue> {
        let tok = self.bump();
        match tok.kind {
            TokenKind::Integer(n) => Ok(PropertyValue::Int(n)),
            TokenKind::Float(f) => Ok(PropertyValue::Float(f)),
            TokenKind::String(s) => Ok(PropertyValue::String(s)),
            TokenKind::Null => Ok(PropertyValue::Null),
            TokenKind::True => Ok(PropertyValue::Bool(true)),
            TokenKind::False => Ok(PropertyValue::Bool(false)),
            other => Err(GraphError::CypherParse {
                line: tok.line,
                col: tok.col,
                message: format!("expected literal, got {other:?}"),
            }),
        }
    }

    fn parse_expr(&mut self) -> GraphResult<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> GraphResult<Expr> {
        let mut left = self.parse_and_expr()?;
        while matches!(self.peek(), TokenKind::Or) {
            self.bump();
            let right = self.parse_and_expr()?;
            left = Expr::Logical {
                op: LogicalOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> GraphResult<Expr> {
        let mut left = self.parse_not_expr()?;
        while matches!(self.peek(), TokenKind::And) {
            self.bump();
            let right = self.parse_not_expr()?;
            left = Expr::Logical {
                op: LogicalOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> GraphResult<Expr> {
        if matches!(self.peek(), TokenKind::Not) {
            self.bump();
            let inner = self.parse_not_expr()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> GraphResult<Expr> {
        // Parenthesised expression.
        if matches!(self.peek(), TokenKind::LParen) {
            self.bump();
            let inner = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(inner);
        }
        // count(...)
        if matches!(self.peek(), TokenKind::Count) {
            self.bump();
            self.expect(TokenKind::LParen)?;
            let inner = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::Count(Box::new(inner)));
        }
        let tok = self.bump();
        match tok.kind {
            TokenKind::Integer(n) => Ok(Expr::Literal(PropertyValue::Int(n))),
            TokenKind::Float(f) => Ok(Expr::Literal(PropertyValue::Float(f))),
            TokenKind::String(s) => Ok(Expr::Literal(PropertyValue::String(s))),
            TokenKind::Null => Ok(Expr::Literal(PropertyValue::Null)),
            TokenKind::True => Ok(Expr::Literal(PropertyValue::Bool(true))),
            TokenKind::False => Ok(Expr::Literal(PropertyValue::Bool(false))),
            TokenKind::Ident(s) => {
                if s.is_empty() {
                    return Err(GraphError::CypherParse {
                        line: tok.line,
                        col: tok.col,
                        message: "unexpected EOF in expression".into(),
                    });
                }
                let mut expr = Expr::Ident(s);
                while matches!(self.peek(), TokenKind::Dot) {
                    self.bump();
                    let prop_tok = self.bump();
                    match prop_tok.kind {
                        TokenKind::Ident(prop) => {
                            expr = Expr::Property {
                                expr: Box::new(expr),
                                prop,
                            };
                        }
                        other => {
                            return Err(GraphError::CypherParse {
                                line: prop_tok.line,
                                col: prop_tok.col,
                                message: format!("expected property name, got {other:?}"),
                            })
                        }
                    }
                }
                // Optionally followed by a binary comparison operator.
                if let Some(op) = self.peek_binary_op() {
                    self.bump();
                    let right = self.parse_not_expr()?;
                    expr = Expr::Binary {
                        op,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                // IS [NOT] NULL
                if matches!(self.peek(), TokenKind::Is) {
                    self.bump();
                    let negated = if matches!(self.peek(), TokenKind::Not) {
                        self.bump();
                        true
                    } else {
                        false
                    };
                    self.expect(TokenKind::Null)?;
                    expr = Expr::IsNull {
                        expr: Box::new(expr),
                        negated,
                    };
                }
                Ok(expr)
            }
            other => Err(GraphError::CypherParse {
                line: tok.line,
                col: tok.col,
                message: format!("unexpected token in expression: {other:?}"),
            }),
        }
    }

    fn peek_binary_op(&self) -> Option<BinaryOp> {
        match self.peek() {
            TokenKind::Eq => Some(BinaryOp::Eq),
            TokenKind::Ne => Some(BinaryOp::Ne),
            TokenKind::Lt => Some(BinaryOp::Lt),
            TokenKind::Le => Some(BinaryOp::Le),
            TokenKind::Gt => Some(BinaryOp::Gt),
            TokenKind::Ge => Some(BinaryOp::Ge),
            _ => None,
        }
    }
}
