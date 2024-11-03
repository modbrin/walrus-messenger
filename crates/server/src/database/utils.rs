pub fn map_not_found_as_none<T>(result: Result<T, sqlx::Error>) -> Result<Option<T>, sqlx::Error> {
    match result {
        Ok(ok) => Ok(Some(ok)),
        Err(e) => {
            if matches!(e, sqlx::Error::RowNotFound) {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}
