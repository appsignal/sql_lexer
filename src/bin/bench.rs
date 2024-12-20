fn long_insert_into_values_query(rows: usize) -> String {
    let mut query = r#"INSERT INTO "table_name" ("one","two","three") VALUES "#.to_owned();
    for i in 0..rows {
        query.push_str(&format!("({}, {}, {}),", i, i + 1, i + 2));
    }
    query.push_str("(0, 0, 0);");
    query
}

extern crate sql_lexer;

fn main() {
    for rows in [200, 2000, 20000] {
        println!("Sanitising long_insert_into_values_query with {rows} rows");
        let query = long_insert_into_values_query(rows);
        let start = std::time::Instant::now();
        let output = sql_lexer::sanitize_string(query);
        let elapsed = start.elapsed();
        println!("Output: {}", output);
        println!("Elapsed: {:?}", elapsed);
    }
}
