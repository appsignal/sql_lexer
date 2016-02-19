use super::{Sql,Token,Operator};

enum State {
    Default,
    ComparisonOperator
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

        for i in 0..self.sql.tokens.len() {
            match self.sql.tokens[i] {
                Token::Operator(Operator::Comparison(_)) => state = State::ComparisonOperator,
                Token::SingleQuoted(_) | Token::DoubleQuoted(_) | Token::Numeric(_) => {
                    match state {
                        State::ComparisonOperator => {
                            self.placeholder(i);
                        },
                        _ => ()
                    }
                    state = State::Default;
                },
                Token::Space => (),
                _ => state = State::Default
            }
        }

        self.sql
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
}
