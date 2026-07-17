use magnus::{function, prelude::*, value::Lazy, Error, ExceptionClass, Ruby, Value};
use sqlparser::{dialect::MySqlDialect, parser::Parser};

static PARSE_ERROR: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.define_module("Finox")
        .unwrap()
        .define_error("ParseError", ruby.exception_standard_error())
        .unwrap()
});

fn parse(ruby: &Ruby, sql: String) -> Result<Value, Error> {
    let statements = Parser::parse_sql(&MySqlDialect {}, &sql)
        .map_err(|e| Error::new(ruby.get_inner(&PARSE_ERROR), e.to_string()))?;
    serde_magnus::serialize(ruby, &statements)
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Finox")?;
    Lazy::force(&PARSE_ERROR, ruby);
    module.define_singleton_method("parse", function!(parse, 1))?;
    Ok(())
}
