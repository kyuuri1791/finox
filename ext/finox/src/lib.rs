use std::collections::HashSet;
use std::fmt::{self, Write as _};
use std::ops::ControlFlow;

use magnus::{function, method, prelude::*, value::Lazy, Error, ExceptionClass, Ruby, Value};
use sqlparser::ast::{AssignmentTarget, Expr, ObjectName, Query, Statement, Visit, Visitor};
use sqlparser::{dialect::MySqlDialect, parser::Parser};

static PARSE_ERROR: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.define_module("Finox")
        .unwrap()
        .define_error("ParseError", ruby.exception_standard_error())
        .unwrap()
});

#[magnus::wrap(class = "Finox::Result", free_immediately, size)]
struct ParseResult {
    statements: Vec<Statement>,
}

impl ParseResult {
    fn statements(ruby: &Ruby, rb_self: &Self) -> Result<Value, Error> {
        let statements = ruby.ary_new_capa(rb_self.statements.len());
        for statement in &rb_self.statements {
            statements.push(ParsedStatement {
                statement: statement.clone(),
            })?;
        }
        Ok(statements.as_value())
    }

    fn tables(&self) -> Vec<String> {
        collect_tables(&self.statements)
    }

    fn columns(&self) -> Vec<String> {
        collect_columns(&self.statements)
    }

    fn statement_types(&self) -> Vec<String> {
        self.statements.iter().map(statement_type).collect()
    }
}

#[magnus::wrap(class = "Finox::Statement", free_immediately, size)]
struct ParsedStatement {
    statement: Statement,
}

impl ParsedStatement {
    fn to_h(ruby: &Ruby, rb_self: &Self) -> Result<Value, Error> {
        serde_magnus::serialize(ruby, &rb_self.statement)
    }

    fn tables(&self) -> Vec<String> {
        collect_tables(&self.statement)
    }

    fn columns(&self) -> Vec<String> {
        collect_columns(&self.statement)
    }

    fn statement_type(&self) -> String {
        statement_type(&self.statement)
    }
}

fn statement_type(statement: &Statement) -> String {
    struct VariantName(String);

    impl fmt::Write for VariantName {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for ch in s.chars() {
                if ch.is_alphanumeric() || ch == '_' {
                    self.0.push(ch);
                } else {
                    return Err(fmt::Error);
                }
            }
            Ok(())
        }
    }

    let mut name = VariantName(String::new());
    let _ = write!(name, "{statement:?}");
    name.0
}

fn collect_tables<T: Visit>(node: &T) -> Vec<String> {
    let mut collector = TableCollector::default();
    let _ = node.visit(&mut collector);

    let relations = collector
        .relations
        .into_iter()
        .filter(|name| !collector.cte_names.contains(name))
        .collect();
    dedup(relations)
}

fn collect_columns<T: Visit>(node: &T) -> Vec<String> {
    let mut collector = ColumnCollector::default();
    let _ = node.visit(&mut collector);
    dedup(collector.columns)
}

fn dedup(names: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for name in names {
        if !result.contains(&name) {
            result.push(name);
        }
    }
    result
}

fn object_name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|part| match part.as_ident() {
            Some(ident) => ident.value.clone(),
            None => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

#[derive(Default)]
struct TableCollector {
    relations: Vec<String>,
    cte_names: HashSet<String>,
}

impl Visitor for TableCollector {
    type Break = ();

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<()> {
        self.relations.push(object_name_to_string(relation));
        ControlFlow::Continue(())
    }

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<()> {
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                self.cte_names.insert(cte.alias.name.value.clone());
            }
        }
        ControlFlow::Continue(())
    }
}

#[derive(Default)]
struct ColumnCollector {
    columns: Vec<String>,
}

impl Visitor for ColumnCollector {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<()> {
        match expr {
            Expr::Identifier(ident) => self.columns.push(ident.value.clone()),
            Expr::CompoundIdentifier(idents) => self.columns.push(
                idents
                    .iter()
                    .map(|ident| ident.value.clone())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_statement(&mut self, statement: &Statement) -> ControlFlow<()> {
        match statement {
            Statement::Insert(insert) => {
                for column in &insert.columns {
                    self.columns.push(object_name_to_string(column));
                }
            }
            Statement::Update(update) => {
                for assignment in &update.assignments {
                    match &assignment.target {
                        AssignmentTarget::ColumnName(name) => {
                            self.columns.push(object_name_to_string(name));
                        }
                        AssignmentTarget::Tuple(names) => {
                            for name in names {
                                self.columns.push(object_name_to_string(name));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

fn parse(ruby: &Ruby, sql: String) -> Result<ParseResult, Error> {
    let statements = Parser::parse_sql(&MySqlDialect {}, &sql)
        .map_err(|e| Error::new(ruby.get_inner(&PARSE_ERROR), e.to_string()))?;
    Ok(ParseResult { statements })
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Finox")?;
    Lazy::force(&PARSE_ERROR, ruby);

    let result = module.define_class("Result", ruby.class_object())?;
    result.define_method("statements", method!(ParseResult::statements, 0))?;
    result.define_method("tables", method!(ParseResult::tables, 0))?;
    result.define_method("columns", method!(ParseResult::columns, 0))?;
    result.define_method("statement_types", method!(ParseResult::statement_types, 0))?;

    let statement = module.define_class("Statement", ruby.class_object())?;
    statement.define_method("to_h", method!(ParsedStatement::to_h, 0))?;
    statement.define_method("tables", method!(ParsedStatement::tables, 0))?;
    statement.define_method("columns", method!(ParsedStatement::columns, 0))?;
    statement.define_method("statement_type", method!(ParsedStatement::statement_type, 0))?;

    module.define_singleton_method("parse", function!(parse, 1))?;
    Ok(())
}
