use std::collections::HashSet;
use std::fmt::{self, Write as _};
use std::ops::ControlFlow;

use magnus::{function, method, prelude::*, value::Lazy, Error, ExceptionClass, Ruby, Value};
use sqlparser::ast::{
    visit_expressions_mut, AssignmentTarget, Expr, FromTable, ObjectName, ObjectType, Query,
    Statement, TableFactor, TableObject, TableWithJoins, Value as SqlValue, Visit, Visitor,
};
use sqlparser::{dialect::MySqlDialect, parser::Parser};
use twox_hash::XxHash64;

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

    fn select_tables(&self) -> Vec<String> {
        collect_select_tables(&self.statements)
    }

    fn dml_tables(&self) -> Vec<String> {
        collect_dml_tables(&self.statements)
    }

    fn ddl_tables(&self) -> Vec<String> {
        collect_ddl_tables(&self.statements)
    }

    fn columns(&self) -> Vec<String> {
        collect_columns(&self.statements)
    }

    fn statement_types(&self) -> Vec<String> {
        self.statements.iter().map(statement_type).collect()
    }

    fn normalize(&self) -> String {
        self.statements
            .iter()
            .map(normalize_statement)
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn fingerprint(&self) -> String {
        fingerprint(&self.normalize())
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

    fn select_tables(&self) -> Vec<String> {
        collect_select_tables(&self.statement)
    }

    fn dml_tables(&self) -> Vec<String> {
        collect_dml_tables(&self.statement)
    }

    fn ddl_tables(&self) -> Vec<String> {
        collect_ddl_tables(&self.statement)
    }

    fn columns(&self) -> Vec<String> {
        collect_columns(&self.statement)
    }

    fn statement_type(&self) -> String {
        statement_type(&self.statement)
    }

    fn normalize(&self) -> String {
        normalize_statement(&self.statement)
    }

    fn fingerprint(&self) -> String {
        fingerprint(&self.normalize())
    }
}

fn fingerprint(sql: &str) -> String {
    format!("{:016x}", XxHash64::oneshot(0, sql.as_bytes()))
}

fn normalize_statement(statement: &Statement) -> String {
    let mut statement = statement.clone();
    let _ = visit_expressions_mut(&mut statement, |expr| {
        if let Expr::Value(value) = expr {
            value.value = SqlValue::Placeholder("?".to_string());
        }
        ControlFlow::<()>::Continue(())
    });
    statement.to_string()
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

fn table_info<T: Visit>(node: &T) -> TableCollector {
    let mut collector = TableCollector::default();
    let _ = node.visit(&mut collector);
    collector
}

fn collect_tables<T: Visit>(node: &T) -> Vec<String> {
    let info = table_info(node);
    let mut names = info.relations;
    names.extend(info.dml_targets);
    names.extend(info.ddl_targets);
    dedup(reject_cte_names(names, &info.cte_names))
}

fn collect_select_tables<T: Visit>(node: &T) -> Vec<String> {
    let info = table_info(node);
    let mut targets = info.dml_targets;
    targets.extend(info.ddl_targets);

    let mut names = Vec::new();
    for name in info.relations {
        match targets.iter().position(|target| *target == name) {
            Some(position) => {
                targets.remove(position);
            }
            None => names.push(name),
        }
    }
    dedup(reject_cte_names(names, &info.cte_names))
}

fn collect_dml_tables<T: Visit>(node: &T) -> Vec<String> {
    dedup(table_info(node).dml_targets)
}

fn collect_ddl_tables<T: Visit>(node: &T) -> Vec<String> {
    dedup(table_info(node).ddl_targets)
}

fn reject_cte_names(names: Vec<String>, cte_names: &HashSet<String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| !cte_names.contains(name))
        .collect()
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

fn table_with_joins_relation(table: &TableWithJoins) -> Option<String> {
    match &table.relation {
        TableFactor::Table { name, .. } => Some(object_name_to_string(name)),
        _ => None,
    }
}

#[derive(Default)]
struct TableCollector {
    relations: Vec<String>,
    dml_targets: Vec<String>,
    ddl_targets: Vec<String>,
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

    fn pre_visit_statement(&mut self, statement: &Statement) -> ControlFlow<()> {
        match statement {
            Statement::Insert(insert) => {
                if let TableObject::TableName(name) = &insert.table {
                    self.dml_targets.push(object_name_to_string(name));
                }
            }
            Statement::Update(update) => {
                if let Some(name) = table_with_joins_relation(&update.table) {
                    self.dml_targets.push(name);
                }
            }
            Statement::Delete(delete) => {
                if delete.tables.is_empty() {
                    let (FromTable::WithFromKeyword(from) | FromTable::WithoutKeyword(from)) =
                        &delete.from;
                    for table in from {
                        if let Some(name) = table_with_joins_relation(table) {
                            self.dml_targets.push(name);
                        }
                    }
                } else {
                    for name in &delete.tables {
                        self.dml_targets.push(object_name_to_string(name));
                    }
                }
            }
            Statement::CreateTable(create) => {
                self.ddl_targets.push(object_name_to_string(&create.name));
            }
            Statement::AlterTable(alter) => {
                self.ddl_targets.push(object_name_to_string(&alter.name));
            }
            Statement::CreateIndex(index) => {
                self.ddl_targets
                    .push(object_name_to_string(&index.table_name));
            }
            Statement::Drop {
                object_type: ObjectType::Table,
                names,
                ..
            } => {
                for name in names {
                    self.ddl_targets.push(object_name_to_string(name));
                }
            }
            Statement::Truncate(truncate) => {
                for target in &truncate.table_names {
                    self.ddl_targets.push(object_name_to_string(&target.name));
                }
            }
            Statement::RenameTable(renames) => {
                for rename in renames {
                    self.ddl_targets.push(object_name_to_string(&rename.old_name));
                    self.ddl_targets.push(object_name_to_string(&rename.new_name));
                }
            }
            _ => {}
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
    result.define_method("select_tables", method!(ParseResult::select_tables, 0))?;
    result.define_method("dml_tables", method!(ParseResult::dml_tables, 0))?;
    result.define_method("ddl_tables", method!(ParseResult::ddl_tables, 0))?;
    result.define_method("columns", method!(ParseResult::columns, 0))?;
    result.define_method("statement_types", method!(ParseResult::statement_types, 0))?;
    result.define_method("normalize", method!(ParseResult::normalize, 0))?;
    result.define_method("fingerprint", method!(ParseResult::fingerprint, 0))?;

    let statement = module.define_class("Statement", ruby.class_object())?;
    statement.define_method("to_h", method!(ParsedStatement::to_h, 0))?;
    statement.define_method("tables", method!(ParsedStatement::tables, 0))?;
    statement.define_method("select_tables", method!(ParsedStatement::select_tables, 0))?;
    statement.define_method("dml_tables", method!(ParsedStatement::dml_tables, 0))?;
    statement.define_method("ddl_tables", method!(ParsedStatement::ddl_tables, 0))?;
    statement.define_method("columns", method!(ParsedStatement::columns, 0))?;
    statement.define_method("statement_type", method!(ParsedStatement::statement_type, 0))?;
    statement.define_method("normalize", method!(ParsedStatement::normalize, 0))?;
    statement.define_method("fingerprint", method!(ParsedStatement::fingerprint, 0))?;

    module.define_singleton_method("parse", function!(parse, 1))?;
    Ok(())
}
