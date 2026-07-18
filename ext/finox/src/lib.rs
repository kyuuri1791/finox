use std::collections::HashSet;
use std::ops::ControlFlow;

use magnus::{function, method, prelude::*, value::Lazy, Error, ExceptionClass, Ruby, Value};
use sqlparser::ast::{ObjectName, Query, Statement, Visit, Visitor};
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
        serde_magnus::serialize(ruby, &rb_self.statements)
    }

    fn tables(&self) -> Vec<String> {
        let mut collector = TableCollector::default();
        let _ = self.statements.visit(&mut collector);

        let mut tables = Vec::new();
        for name in collector.relations {
            if !collector.cte_names.contains(&name) && !tables.contains(&name) {
                tables.push(name);
            }
        }
        tables
    }
}

#[derive(Default)]
struct TableCollector {
    relations: Vec<String>,
    cte_names: HashSet<String>,
}

impl Visitor for TableCollector {
    type Break = ();

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<()> {
        let name = relation
            .0
            .iter()
            .map(|part| match part.as_ident() {
                Some(ident) => ident.value.clone(),
                None => part.to_string(),
            })
            .collect::<Vec<_>>()
            .join(".");
        self.relations.push(name);
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

    module.define_singleton_method("parse", function!(parse, 1))?;
    Ok(())
}
