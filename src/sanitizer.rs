use super::{Sql,Token,Keyword};

#[derive(Debug,PartialEq)]
enum State {
    Default,
    ComparisonOperator,
    ComparisonScopeStarted,
    InsertValues,
    JoinOn,
    Offset,
    Between
}

pub struct SqlSanitizer {
    pub sql: Sql
}

impl SqlSanitizer {
    pub fn new(sql: Sql) -> SqlSanitizer {
        SqlSanitizer {
            sql: sql
        }
    }

    pub fn sanitize(mut self) -> Sql {
        let mut state = State::Default;

        let mut pos = 0;
        loop {
            if pos >= self.sql.tokens.len() {
                break
            }

            match self.sql.tokens[pos] {
                // Determine if we want to change or keep state
                Token::Operator(_) if state != State::JoinOn => state = State::ComparisonOperator,
                Token::Keyword(Keyword::Values) => state = State::InsertValues,
                Token::Keyword(Keyword::On) => state = State::JoinOn,
                Token::Keyword(Keyword::Offset) => state = State::Offset,
                Token::Keyword(Keyword::Between) => state = State::Between,
                Token::Keyword(Keyword::And) if state == State::Between => (),
                Token::ParentheseOpen if state == State::ComparisonOperator => state = State::ComparisonScopeStarted,
                Token::ParentheseOpen if state == State::InsertValues => (),
                Token::Comma if state == State::InsertValues => (),
                Token::Dot if state == State::JoinOn => (),
                // This is content we might want to sanitize
                Token::SingleQuoted(_) | Token::DoubleQuoted(_) | Token::Numeric(_) | Token::Null => {
                    match state {
                        State::ComparisonOperator | State::Offset | State::Between => {
                            // We're after a comparison operator, offset or between/and, so insert placeholder.
                            self.placeholder(pos);
                        },
                        State::ComparisonScopeStarted => {
                            // We're in an IN () and it starts with content. Remove everything until
                            // the closing parenthese and put one placeholder in between.
                            let start_pos = pos;
                            loop {
                                if pos >= self.sql.tokens.len() {
                                    break
                                }
                                match self.sql.tokens[pos] {
                                    Token::ParentheseClose => break,
                                    _ => self.remove(pos)
                                }
                            }
                            self.sql.tokens.insert(start_pos, Token::Placeholder);
                        },
                        State::InsertValues => {
                            // We're in an insert block, insert placeholder.
                            self.placeholder(pos);
                        },
                        _ => ()
                    }
                },
                // Strip comments
                Token::Comment(_) => {
                    self.remove(pos);
                    // Remove preceding space if there is one
                    if pos > 1 && self.sql.tokens[pos - 1] == Token::Space {
                        self.remove(pos - 1);
                        continue;
                    }
                },
                // Spaces don't influence the state
                Token::Space => (),
                // Reset state to default if there were no matches
                _ => state = State::Default
            }

            pos += 1;
        }

        self.sql
    }

    fn remove(&mut self, position: usize) {
        self.sql.tokens.remove(position);
    }

    fn placeholder(&mut self, position: usize) {
        self.sql.tokens.remove(position);
        self.sql.tokens.insert(position, Token::Placeholder);
    }
}

#[cfg(test)]
mod tests {
    use super::super::sanitize_string;

    #[test]
    fn test_select_where_single_quote() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 'secret' LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
        );
    }

    #[test]
    fn test_select_where_double_quote() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = \"secret\" LIMIT 1;".to_string()),
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
            sanitize_string("SELECT `table`.* FROM `table` WHERE `field` BETWEEN 5 AND 10;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `field` BETWEEN ? AND ?;"
        );
    }

    #[test]
    fn test_select_and_with_scope_and_unquoted_field() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 1 AND (other_field = 1) LIMIT 1;".to_string()),
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
    fn test_select_in() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` IN (1, 2, 3) LIMIT 1;".to_string()),
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
    fn test_select_with_functions_regex_and_newlines() {
        let original =  "SELECT a.attname, format_type(a.atttypid, a.atttypmod),
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

        assert_eq!(
            sanitize_string(original.to_string()),
            sanitized
        );
    }

    #[test]
    fn test_update_backquote_tables() {
        assert_eq!(
            sanitize_string("UPDATE `table` SET `field` = \"value\", `field2` = 1 WHERE id = 1;".to_string()),
            "UPDATE `table` SET `field` = ?, `field2` = ? WHERE id = ?;"
        );
    }

    #[test]
    fn test_update_double_quote_tables() {
        assert_eq!(
            sanitize_string("UPDATE \"table\" SET \"field1\" = 'value', \"field2\" = 1 WHERE id = 1;".to_string()),
            "UPDATE \"table\" SET \"field1\" = ?, \"field2\" = ? WHERE id = ?;"
        );
    }

    #[test]
    fn test_insert_backquote_tables() {
        assert_eq!(
            sanitize_string("INSERT INTO `table` (`field1`, `field2`) VALUES ('value', 1);".to_string()),
            "INSERT INTO `table` (`field1`, `field2`) VALUES (?, ?);"
        );
    }

    #[test]
    fn test_insert_doublequote_tables() {
        assert_eq!(
            sanitize_string("INSERT INTO \"table\" (\"field1\", \"field2\") VALUES ('value', 1);".to_string()),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?);"
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
            sanitize_string("INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (NULL, 1);".to_string()),
            "INSERT INTO \"table\" (\"field1\", \"field2\") VALUES (?, ?);"
        );
    }

    #[test]
    fn test_comment_pound() {
        assert_eq!(
            sanitize_string("SELECT * FROM table # This is a comment".to_string()),
            "SELECT * FROM table"
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
    fn test_keep_placeholders() {
        let sql = "SELECT \"users\".* FROM \"users\" WHERE \"users\".\"type\" IN (?) AND \"users\".\"active\" = $1";

        assert_eq!(
            sanitize_string(sql.to_string()),
            sql
        );
    }
}
