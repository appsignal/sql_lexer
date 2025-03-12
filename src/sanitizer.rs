use super::{Keyword, Sql, Token};

#[derive(Debug, PartialEq)]
enum State {
    Default,
    ComparisonOperator,
    ComparisonScopeStarted,
    InsertValues,
    InsertValuesJustClosed,
    JoinOn,
    Offset,
    Between,
    Keyword,
    KeywordScopeStarted,
    Array,
    ArrayStarted,
}

pub struct SqlSanitizer {
    pub sql: Sql,
}

impl SqlSanitizer {
    pub fn new(sql: Sql) -> SqlSanitizer {
        SqlSanitizer { sql }
    }

    pub fn sanitize(mut self) -> Sql {
        let mut state = State::Default;

        let mut pos = 0;
        loop {
            if pos >= self.sql.tokens.len() {
                break;
            }

            match &self.sql.tokens[pos] {
                // Determine if we want to change or keep state
                Token::Operator(_) if state != State::JoinOn => state = State::ComparisonOperator,
                Token::Keyword(Keyword::Values) => state = State::InsertValues,
                Token::Keyword(Keyword::On) => state = State::JoinOn,
                Token::Keyword(Keyword::Offset) => state = State::Offset,
                Token::Keyword(Keyword::Between) => state = State::Between,
                Token::Keyword(Keyword::Array) => state = State::Array,
                Token::Keyword(Keyword::And) if state == State::Between => (),
                Token::Keyword(Keyword::And) if state == State::Keyword => {
                    state = State::KeywordScopeStarted
                }
                Token::Keyword(Keyword::Or) if state == State::Keyword => {
                    state = State::KeywordScopeStarted
                }
                Token::Keyword(Keyword::Insert) | Token::Keyword(Keyword::Into) => (),
                Token::Keyword(_) if state == State::KeywordScopeStarted => {
                    state = State::KeywordScopeStarted
                }
                Token::Keyword(_) => state = State::Keyword,
                Token::LiteralValueTypeIndicator(_) => (),
                Token::ParentheseOpen if state == State::ComparisonOperator => {
                    state = State::ComparisonScopeStarted
                }
                Token::ParentheseOpen if state == State::Keyword => {
                    state = State::KeywordScopeStarted
                }
                Token::ParentheseOpen if state == State::InsertValues => (),
                Token::SquareBracketOpen if state == State::Array => state = State::ArrayStarted,
                Token::ParentheseClose if state == State::InsertValues => {
                    state = State::InsertValuesJustClosed
                }
                Token::Comma if state == State::InsertValuesJustClosed => (),
                Token::ParentheseOpen if state == State::InsertValuesJustClosed => {
                    // We're past the first insert values clause. Remove every parentheses
                    // group and replace them with an ellipsis.
                    let start_pos = pos;
                    let mut in_parentheses = true;
                    let mut kept = 0;

                    loop {
                        if pos + kept >= self.sql.tokens.len() {
                            break;
                        }

                        let keep = match self.sql.tokens[pos + kept] {
                            Token::ParentheseClose => {
                                in_parentheses = false;
                                false
                            }
                            Token::ParentheseOpen => {
                                in_parentheses = true;
                                false
                            }
                            _ if in_parentheses => false,
                            Token::Comma => true,
                            Token::Space => true,
                            _ => break,
                        };

                        if keep {
                            kept += 1;
                        } else {
                            let mut removed = 1;
                            self.remove(pos + kept);
                            while kept > 0 {
                                kept -= 1;
                                removed += 1;
                                self.remove(pos + kept);
                            }
                            pos += removed;
                        }
                    }
                    self.ellipsis(start_pos);
                }
                Token::ParentheseClose | Token::SquareBracketClose => state = State::Default,
                Token::Dot if state == State::JoinOn => (),
                // This is content we might want to sanitize
                token @ (Token::SingleQuoted(_)
                | Token::DoubleQuoted(_)
                | Token::Numeric(_)
                | Token::Null
                | Token::True
                | Token::False) => {
                    match state {
                        State::ComparisonOperator
                        | State::InsertValues
                        | State::Offset
                        | State::KeywordScopeStarted
                        | State::Between => {
                            // Double quoted might (standard SQL) or might not (MySQL) be an identifier,
                            // but if it's a component in a dotted path, then we know it's part of an
                            // identifier and we should definitely not replace it with a placeholder.
                            match token {
                                Token::DoubleQuoted(_) => {
                                    if !(self.sql.tokens.get(pos - 1) == Some(&Token::Dot)
                                        || self.sql.tokens.get(pos + 1) == Some(&Token::Dot))
                                    {
                                        self.placeholder(pos)
                                    }
                                }
                                _ => self.placeholder(pos),
                            }
                        }
                        State::ComparisonScopeStarted | State::ArrayStarted => {
                            // We're in an IN () or ARRAY[] and it starts with content. Remove everything until
                            // the closing parenthese and put one placeholder in between.
                            let start_pos = pos;
                            loop {
                                if pos >= self.sql.tokens.len() {
                                    break;
                                }
                                match self.sql.tokens[pos] {
                                    Token::ParentheseClose => break,
                                    Token::SquareBracketClose => break,
                                    _ => {
                                        self.remove(pos);
                                        pos += 1;
                                    }
                                }
                            }
                            self.placeholder(start_pos);
                        }
                        _ => (),
                    }
                }
                // Remove comments
                Token::Comment(_) => {
                    self.remove(pos);
                    if self.sql.tokens.get(pos - 1) == Some(&Token::Space) {
                        self.remove(pos - 1);
                    }
                }
                // Spaces don't influence the state by default
                Token::Space => (),
                // Non-tokens don't influence the state
                Token::None => (),
                // Keep state the same if we're in a insert values or keyword scope state
                _ if state == State::InsertValues || state == State::KeywordScopeStarted => (),
                // Reset state to default if there were no matches
                _ => state = State::Default,
            }

            pos += 1;
        }

        self.sql
    }

    fn remove(&mut self, position: usize) {
        self.sql.tokens[position] = Token::None;
    }

    // Replaces the token at position `position` with a placeholder.
    fn placeholder(&mut self, position: usize) {
        self.sql.tokens[position] = Token::Placeholder;
    }

    // Replaces the token at position `position` with an ellipsis.
    fn ellipsis(&mut self, position: usize) {
        self.sql.tokens[position] = Token::Ellipsis;
    }
}

#[cfg(test)]
mod tests {
    use super::super::sanitize_string;

    #[test]
    fn test_empty() {
        assert_eq!(sanitize_string("".to_string()), "")
    }

    #[test]
    fn test_select_where_single_quote() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `id` = 'secret' LIMIT 1;".to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_double_quote() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `id` = \"secret\" LIMIT 1;".to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_table_name_no_quotes() {
        assert_eq!(
            sanitize_string("SELECT table.* FROM table WHERE id = 'secret' LIMIT 1;".to_string()),
            "SELECT table.* FROM table WHERE id = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_numeric() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 1 LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_numeric_negative() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = -1 LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_with_function() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `name` = UPPERCASE('lower') LIMIT 1;"
                    .to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `name` = UPPERCASE(?) LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_with_function_multiple_args() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `name` = COMMAND('table', 'lower') LIMIT 1;"
                    .to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `name` = COMMAND(?, ?) LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_with_function_mixed_args() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `name` = COMMAND(`table`, 'lower') LIMIT 1;"
                    .to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `name` = COMMAND(`table`, ?) LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_with_nested_function() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `name` = LOWERCASE(UPPERCASE('lower')) LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `name` = LOWERCASE(UPPERCASE(?)) LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_like() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` LIKE 'value'".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` LIKE ?"
        );
    }

    #[test]
    fn test_select_limit_and_offset() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` LIMIT 10 OFFSET 5;".to_string()),
            "SELECT `table`.* FROM `table` LIMIT 10 OFFSET ?;"
        );
    }

    #[test]
    fn test_select_and_quoted() {
        assert_eq!(
            sanitize_string("SELECT \"table\".* FROM \"table\" WHERE \"field1\" = 1 AND \"field2\" = 'something';".to_string()),
            "SELECT \"table\".* FROM \"table\" WHERE \"field1\" = ? AND \"field2\" = ?;"
        );
    }

    #[test]
    fn test_select_between_and() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `field` BETWEEN 5 AND 10;".to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `field` BETWEEN ? AND ?;"
        );
    }

    #[test]
    fn test_select_and_with_scope_and_unquoted_field() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `id` = 1 AND (other_field = 1) LIMIT 1;"
                    .to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `id` = ? AND (other_field = ?) LIMIT 1;"
        );
    }

    #[test]
    fn test_count_start() {
        assert_eq!(
            sanitize_string("SELECT COUNT(*) FROM `table` WHERE `field` = 1;".to_string()),
            "SELECT COUNT(*) FROM `table` WHERE `field` = ?;"
        );
    }

    #[test]
    fn test_select_where_already_placeholder() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = $1 LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` = $1 LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_or_and_operators() {
        assert_eq!(
            sanitize_string("SELECT `posts`.* FROM `posts` WHERE (created_at >= '2016-01-10 13:34:46.647328' OR updated_at >= '2016-01-10 13:34:46.647328')".to_string()),
            "SELECT `posts`.* FROM `posts` WHERE (created_at >= ? OR updated_at >= ?)"
        );
    }

    #[test]
    fn test_select_reversed_comparison_operators() {
        assert_eq!(
            sanitize_string("SELECT `posts`.* FROM `posts` WHERE ('2016-01-10' >= created_at AND '2016-01-10' <= updated_at OR '2021-10-22' = published_at)".to_string()),
            "SELECT `posts`.* FROM `posts` WHERE (? >= created_at AND ? <= updated_at OR ? = published_at)"
        );
    }

    #[test]
    fn test_bitfield_modifier() {
        assert_eq!(
            sanitize_string("SELECT * FROM `posts` WHERE `field` = x'42'".to_string()),
            "SELECT * FROM `posts` WHERE `field` = x?"
        )
    }

    #[test]
    fn test_date_modifier() {
        assert_eq!(
            sanitize_string(
                "SELECT * FROM `posts` WHERE `field` = DATE 'str' AND `field2` = DATE'str'"
                    .to_string()
            ),
            "SELECT * FROM `posts` WHERE `field` = DATE ? AND `field2` = DATE?"
        )
    }

    #[test]
    fn test_binary_modifier() {
        assert_eq!(
            sanitize_string("SELECT * FROM `posts` WHERE `field` = BINARY '123' and `field2` = BINARY'456' AND `field3` = BINARY 789".to_string()),
            "SELECT * FROM `posts` WHERE `field` = BINARY ? AND `field2` = BINARY? AND `field3` = BINARY ?"
        )
    }

    #[test]
    fn test_string_modifier() {
        assert_eq!(
            sanitize_string(
                "SELECT * FROM `posts` WHERE `field` = n'str' AND `field2` = _utf8'str'"
                    .to_string()
            ),
            "SELECT * FROM `posts` WHERE `field` = n? AND `field2` = _utf8?"
        )
    }

    #[test]
    fn test_select_in() {
        assert_eq!(
            sanitize_string(
                "SELECT `table`.* FROM `table` WHERE `id` IN (1, 2, 3) LIMIT 1;".to_string()
            ),
            "SELECT `table`.* FROM `table` WHERE `id` IN (?) LIMIT 1;"
        );
    }

    #[test]
    fn test_select_in_subquery() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` IN (SELECT `id` FROM `something` WHERE `a` = 1) LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` IN (SELECT `id` FROM `something` WHERE `a` = ?) LIMIT 1;"
        );
    }

    #[test]
    fn test_solidcache_json() {
        assert_eq!(
            sanitize_string("INSERT INTO `solid_cache_entries` (`key`,`value`,`created_at`) VALUES (x'76696577732f6974656d732f696e6465783a38323536366332313963306634393838626133333533366630623233623336382f6974656d732f31', '{\"p\":\"/CGjPP3XaEcnxN1hrPEId9VPX50YLDVTCyrZp9m1E+NBuJnfBGu2pEY2DzwaWcMCDdY1m2+IPH2mPqjT11Q3sdHx+UA1jTDzW1kHAEDfbwJjZU4sRh5+CKJqcvtB5+pP6gDIOaBKQdBl6sroZ+uODSy9LGMKJlQElaiqOmeLeOHH/UOX3aGeLpjSkRkPglBuAgsmqqmBAz8TlEzKRzIu+L6O8KL/tNJwLbFA8yU4IuT4t+a4rSqLK6+rZDwnAC1i+n6HH50i0KL/1xjKhC+oZ6g+fp9qQRyPOs9L1ffD670jMTspBvQ8A1QkOsn21hnWQGQlxKL+VWRu2Zxd5mhSj++xTL9WQwuhCHAClzs6oSbNGWM8W0uVutDwkASdEg9BNfpu0gVG/YbVkn0/EFqSoA5/YaR3HCKV1EeCNeffbG0sxXF6YVaMLzr+z7xwrBUhIR5VthOcvSRbla/Ww0yBDJpszDlsV/3aMtKMgFE572dD0mlwF2xALBOFJ3xbXGHML72Q/gOOtnlS8VlBJzD1Lklb/WY/SNLL7m0clUAHZxU/9wbWRnnlyWKymZdtym6xI49b1GeGv210jvgApVsaisweaFNsJ7fEEU7xFoBcxXKUuX503GaGUhttS1jALTrIdCatmh4FTEKJ47At2DgjoL35OP2ZlNQdQ3dI+tnsiOR/vbEgZ935eXfYsJPa79BH0zNwvi3ggqnXTYzaF92PY4cd0LO9sT7YZEbrM0KUcaXvTP63HC6tfGbRct73IRWfmIbXf5GFP6/m6CCh6LpoZP6pIx9cWqfndAe5/7QpL7A5h6MqZGy/5wbsUWXaIiSYbu2s+54iVYytYaO9in5fhAYIIFydsr4bQrPewFI6wzBO641DEKKiovC857D3yfgFc30Qob0FrUZq69kABI8sYsPctkIYfIcSXOUr7ZZdvb7+d9+uRIY0zncPc9hvYFqxY4MmQA7JQutGiDj/fJ+04xoTJBJB2tJ0gm4KnDjwv5oQUHGvYYT/ihZu98PTcPZmnmcoz6m3SoVd4T+IjQB/O/ay+r1mlTP9/ZdaI2bzds7XHb15SybmH3SbNccRP3JzXPYpvrX5syLdCykwsWppOki+pXHj6hT/peCay7KuxpQBj56USfuRT0ieXiRWUfirq/LrEEPLTAxgAFvEm7tdbGcjllOW8sSRyvFtredmT52qZdJGpHgkMewYQ5fGmpyQgmsuor2ahjHoZ9PFRxRjyZpslrcpm4UIrZklHHskiTh7YA5txebBMP8hmA==\",\"h\":{\"iv\":\"lwHVE2obcSHlpKoO\",\"at\":\"8wKr3ZuAMj6hgKmkL2iIQg==\",\"e\":\"QVNDSUktOEJJVA==\",\"c\":true}}', CURRENT_TIMESTAMP(6)) ON DUPLICATE KEY UPDATE `value`=VALUES(`value`)".to_string()),
            "INSERT INTO `solid_cache_entries` (`key`,`value`,`created_at`) VALUES (x?, ?, CURRENT_TIMESTAMP(?)) ON DUPLICATE KEY UPDATE `value`=VALUES(`value`)"
        );
    }

    #[test]
    fn test_case_then_else_subquery() {
        assert_eq!(
            sanitize_string(
                "CASE WHEN NOT EXISTS (SELECT * FROM `table` WHERE `id` = 1) THEN 1 ELSE '0' END;"
                    .to_string()
            ),
            "CASE WHEN NOT EXISTS (SELECT * FROM `table` WHERE `id` = ?) THEN ? ELSE ? END;"
        );
    }

    #[test]
    fn test_select_array() {
        assert_eq!(
            sanitize_string(
                "SELECT * FROM \"table\" WHERE \"field\" = ARRAY['item_1','item_2','item_3'];"
                    .to_string()
            ),
            "SELECT * FROM \"table\" WHERE \"field\" = ARRAY[?];"
        );
    }

    #[test]
    fn test_select_join_backquote_tables() {
        assert_eq!(
            sanitize_string("SELECT * FROM `tables` INNER JOIN `other` ON `table`.`id` = `other`.`table_id` WHERE `other`.`field` = 1);".to_string()),
            "SELECT * FROM `tables` INNER JOIN `other` ON `table`.`id` = `other`.`table_id` WHERE `other`.`field` = ?);"
        );
    }

    #[test]
    fn test_select_join_doublequote_tables() {
        assert_eq!(
            sanitize_string("SELECT * FROM \"tables\" INNER JOIN \"other\" ON \"table\".\"id\" = \"other\".\"table_id\" WHERE \"other\".\"field\" = 1);".to_string()),
            "SELECT * FROM \"tables\" INNER JOIN \"other\" ON \"table\".\"id\" = \"other\".\"table_id\" WHERE \"other\".\"field\" = ?);"
        );
    }

    #[test]
    fn test_select_order_by_field() {
        assert_eq!(
            sanitize_string(
                r#"SELECT `table`.* FROM `table` WHERE `table`.`id` = 1 ORDER BY field(id, 111030,1933535)"#.to_string()
            ),
            r#"SELECT `table`.* FROM `table` WHERE `table`.`id` = ? ORDER BY field(?)"#
        );
    }

    #[test]
    fn test_select_with_functions_regex_and_newlines() {
        let original = "SELECT a.attname, format_type(a.atttypid, a.atttypmod),
                         pg_get_expr(d.adbin, d.adrelid), a.attnotnull, a.atttypid, a.atttypmod
                         FROM pg_attribute a LEFT JOIN pg_attrdef d
                         ON a.attrelid = d.adrelid AND a.attnum = d.adnum
                         WHERE a.attrelid = '\"value\"'::regclass
                         AND a.attnum > 0 AND NOT a.attisdropped
                         ORDER BY a.attnum;";

        let sanitized = "SELECT a.attname, format_type(a.atttypid, a.atttypmod),
                         pg_get_expr(d.adbin, d.adrelid), a.attnotnull, a.atttypid, a.atttypmod
                         FROM pg_attribute a LEFT JOIN pg_attrdef d
                         ON a.attrelid = d.adrelid AND a.attnum = d.adnum
                         WHERE a.attrelid = ?::regclass
                         AND a.attnum > ? AND NOT a.attisdropped
                         ORDER BY a.attnum;";

        assert_eq!(sanitize_string(original.to_string()), sanitized);
    }

    #[test]
    fn test_update_backquote_tables() {
        assert_eq!(
            sanitize_string(
                "UPDATE `table` SET `field` = \"value\", `field2` = 1 WHERE id = 1;".to_string()
            ),
            "UPDATE `table` SET `field` = ?, `field2` = ? WHERE id = ?;"
        );
    }

    #[test]
    fn test_update_double_quote_tables() {
        assert_eq!(
            sanitize_string(
                "UPDATE \"table\" SET \"field1\" = 'value', \"field2\" = 1 WHERE id = 1;"
                    .to_string()
            ),
            "UPDATE \"table\" SET \"field1\" = ?, \"field2\" = ? WHERE id = ?;"
        );
    }

    #[test]
    fn test_insert_backquote_tables() {
        assert_eq!(
            sanitize_string(
                "INSERT INTO `table` (`field1`, `field2`) VALUES ('value', 1, -1.0, 'value');"
                    .to_string()
            ),
            "INSERT INTO `table` (`field1`, `field2`) VALUES (?, ?, ?, ?);"
        );
    }

    #[test]
    fn test_insert_doublequote_tables() {
        assert_eq!(
            sanitize_string("INSERT INTO \"table\" (\"field1\", \"field2\") VALUES ('value', 1, -1.0, 'value');".to_string()),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?, ?, ?);"
        );
    }

    #[test]
    fn test_insert_multiple_values() {
        assert_eq!(
            sanitize_string("INSERT INTO `table` (`field1`, `field2`) VALUES ('value', 1, -1.0, 'value'),('value', 1, -1.0, 'value'),('value', 1, -1.0, 'value');".to_string()),
            "INSERT INTO `table` (`field1`, `field2`) VALUES (?, ?, ?, ?),...;"
        );
    }

    #[test]
    fn test_insert_multiple_values_with_spaces() {
        assert_eq!(
            sanitize_string("INSERT INTO `table` (`field1`, `field2`) VALUES ('value', 1, -1.0, 'value'), ('value', 1, -1.0, 'value'), ('value', 1, -1.0, 'value');".to_string()),
            "INSERT INTO `table` (`field1`, `field2`) VALUES (?, ?, ?, ?), ...;"
        );
    }

    #[test]
    fn test_insert_returning() {
        assert_eq!(
            sanitize_string("INSERT INTO \"table\" (\"field1\", \"field2\") VALUES ('value', 1) RETURNING \"id\";".to_string()),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?) RETURNING \"id\";"
        );
    }

    #[test]
    fn test_insert_null() {
        assert_eq!(
            sanitize_string(
                "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (1, NULL), (NULL, 1), (NULL, NULL), (1, 1);".to_string()
            ),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?), ...;"
        );
    }

    #[test]
    fn test_insert_true_false() {
        assert_eq!(
            sanitize_string(
                "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (TRUE, FALSE), (1, 2), (FALSE, TRUE), (2, 1);".to_string()
            ),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?), ...;"
        );
    }

    #[test]
    fn test_insert_many_on_conflict() {
        assert_eq!(
            sanitize_string(
                "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (TRUE, FALSE), (1, 2), (FALSE, TRUE), (2, 1) ON CONFLICT DO NOTHING RETURNING \"field1\";".to_string()
            ),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?), ... ON CONFLICT DO NOTHING RETURNING \"field1\";"
        );
    }

    #[test]
    fn test_comment_pound() {
        assert_eq!(
            sanitize_string("SELECT * FROM table # This is a comment\n SELECT".to_string()),
            "SELECT * FROM table\n SELECT"
        );
    }

    #[test]
    fn test_comment_double_dash() {
        assert_eq!(
            sanitize_string("SELECT * FROM table -- This is a comment\n SELECT".to_string()),
            "SELECT * FROM table\n SELECT"
        );
    }

    #[test]
    fn test_comment_multi_line() {
        assert_eq!(
            sanitize_string("SELECT * FROM table /* This is a comment */ SELECT".to_string()),
            "SELECT * FROM table SELECT"
        );
    }

    #[test]
    fn test_comment_end_of_subquery() {
        assert_eq!(
            sanitize_string("SELECT COUNT(*) FROM (SELECT (*) from table WHERE table.attr = 123 /* traceparent=00-a7bd9142c227de0d3c1dccb3a21800b8-1e30b841ea8c9b77-01 */) AS 'sub'".to_string()),
            "SELECT COUNT(*) FROM (SELECT (*) FROM table WHERE table.attr = ?) AS 'sub'"
        );
    }

    #[test]
    fn test_keep_placeholders() {
        let sql = "SELECT \"users\".* FROM \"users\" WHERE \"users\".\"type\" IN (?) AND \"users\".\"active\" = $1";

        assert_eq!(sanitize_string(sql.to_string()), sql);
    }

    #[test]
    fn test_json_operations() {
        assert_eq!(
            sanitize_string(
                "SELECT table.*, NULLIF((table2.json_col #>> '{obj1,obj2}')::float, 0) FROM table"
                    .to_string()
            ),
            "SELECT table.*, NULLIF((table2.json_col #>> ?)::float, 0) FROM table"
        )
    }

    #[test]
    fn test_remove_trailing_comments_multiline() {
        assert_eq!(
            sanitize_string("SELECT table.* FROM table; /* trace: a1b2c3d4e5f6 */".to_string()),
            "SELECT table.* FROM table;"
        );
    }

    #[test]
    fn test_remove_trailing_comments_inline() {
        assert_eq!(
            sanitize_string("SELECT table.* FROM table; -- trace: a1b2c3d4e5f6".to_string()),
            "SELECT table.* FROM table;"
        );
    }

    #[test]
    fn test_remove_trailing_comments_before_semicolon() {
        assert_eq!(
            sanitize_string("SELECT table.* FROM table /* trace: a1b2c3d4e5f6 */;".to_string()),
            "SELECT table.* FROM table;"
        );
    }

    #[test]
    fn test_select_jsonb_extract_path() {
        assert_eq!(
            sanitize_string(
                "SELECT jsonb_extract_path(table.data, 'foo', 22) FROM table;".to_string()
            ),
            "SELECT jsonb_extract_path(table.data, ?, ?) FROM table;"
        );
    }

    #[test]
    fn test_select_jsonb_extract_path_quoted_identifier() {
        assert_eq!(
            sanitize_string(
                "SELECT jsonb_extract_path(\"table\".\"data\", 'foo', 22) FROM \"table\";"
                    .to_string()
            ),
            "SELECT jsonb_extract_path(\"table\".\"data\", ?, ?) FROM \"table\";"
        );
    }

    #[test]
    fn test_where_jsonb_extract_path() {
        assert_eq!(
            sanitize_string(
                "SELECT id FROM table WHERE jsonb_extract_path(table.data, 'foo', 22) = 'bar';"
                    .to_string()
            ),
            "SELECT id FROM table WHERE jsonb_extract_path(table.data, ?, ?) = ?;"
        );
    }

    #[test]
    fn test_where_quoted_identifier_in_parenthesis() {
        assert_eq!(
            sanitize_string(
                r#"SELECT "table"."id" FROM "table" WHERE ("table"."data" = 'foo');"#.to_string()
            ),
            r#"SELECT "table"."id" FROM "table" WHERE ("table"."data" = ?);"#
        );
    }
}
