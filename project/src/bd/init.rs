use crate::bd::connection::DbContext;
use once_cell::sync::OnceCell;
use std::sync::Arc;

static DB_CONTEXT: OnceCell<Arc<DbContext>> = OnceCell::new();

pub fn init_db(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let context = Arc::new(DbContext::open(path)?);
    context.init_schema()?;
    DB_CONTEXT
        .set(context)
        .map_err(|_| "Database context already initialized".into())
}

pub fn get_db() -> Arc<DbContext> {
    DB_CONTEXT
        .get()
        .expect("Database context not initialized. Call init_db() first.")
        .clone()
}

#[cfg(test)]
pub fn create_test_db(path: Option<&str>) -> Result<DbContext, Box<dyn std::error::Error>> {
    let context = DbContext::open(path)?;
    context.init_schema()?;
    Ok(context)
}
