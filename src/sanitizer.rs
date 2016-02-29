use super::{Sql,Token,Keyword,Operator};

#[derive(Debug,PartialEq)]
enum State {
    Default,
    ComparisonOperator,
    In,
    InStarted,
    InsertValues,
    InsertValuesStarted
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
                Token::Operator(Operator::Comparison(_)) => state = State::ComparisonOperator,
                Token::Keyword(Keyword::In) => state = State::In,
                Token::Keyword(Keyword::Values) => state = State::InsertValues,
                Token::SingleQuoted(_) | Token::DoubleQuoted(_) | Token::Numeric(_) => {
                    match state {
                        State::ComparisonOperator => {
                            // We're after a comparison operator, so insert placeholder.
                            self.placeholder(pos);
                        },
                        State::InStarted => {
                            // We're in an IN () and it starts with content. Remove everything until
                            // the closing parenthese and put one placeholder in between.
                            let start_pos = pos;
                            loop {
                                if pos >= self.sql.tokens.len() {
                                    break
                                }
                                match self.sql.tokens[pos] {
                                    Token::Operator(Operator::ParentheseClose) => break,
                                    _ => self.remove(pos)
                                }
                            }
                            self.sql.tokens.insert(start_pos, Token::Placeholder);
                        },
                        State::InsertValuesStarted => {
                            // We're in an insert block, insert placeholder.
                            self.placeholder(pos);
                        },
                        _ => ()
                    }
                },
                Token::Operator(Operator::ParentheseOpen) if state == State::In => state = State::InStarted,
                Token::Operator(Operator::ParentheseOpen) if state == State::InsertValues => state = State::InsertValuesStarted,
                Token::Operator(Operator::Comma) if state == State::InsertValuesStarted => (), // If this is a , in a values block keep state
                Token::Space => (),
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
    fn test_select_where_numeric() {
        assert_eq!(
            sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 1 LIMIT 1;".to_string()),
            "SELECT `table`.* FROM `table` WHERE `id` = ? LIMIT 1;"
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
}
