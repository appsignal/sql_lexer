use super::{Keyword,Token,Operator,ComparisonOperator,Sql};

pub struct SqlWriter {
    pub sql: Sql
}

impl SqlWriter {
    pub fn new(sql: Sql) -> SqlWriter {
        SqlWriter {
            sql: sql
        }
    }

    pub fn write(&self) -> String {
        let mut out = String::new();

        for token in self.sql.tokens.iter() {
            match token {
                &Token::Keyword(Keyword::Select) => out.push_str("SELECT"),
                &Token::Keyword(Keyword::From) => out.push_str("FROM"),
                &Token::Keyword(Keyword::Where) => out.push_str("WHERE"),
                &Token::Keyword(Keyword::And) => out.push_str("AND"),
                &Token::Keyword(Keyword::In) => out.push_str("IN"),
                &Token::Keyword(Keyword::Other(ref pos)) => {
                    out.push_str(self.sql.buffer_content(pos));
                },
                &Token::Operator(Operator::Dot) => out.push('.'),
                &Token::Operator(Operator::Multiply) => out.push('*'),
                &Token::Operator(Operator::Comparison(ComparisonOperator::Equal)) => out.push('='),
                &Token::Operator(Operator::Comparison(ComparisonOperator::NullSafeEqual)) => out.push_str("<=>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)) => out.push_str(">="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)) => out.push_str("<="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrGreaterThan)) => out.push_str("=>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrLessThan)) => out.push_str("<="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualWithArrows)) => out.push_str("<>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::NotEqual)) => out.push_str("!="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)) => out.push('>'),
                &Token::Operator(Operator::Comparison(ComparisonOperator::LessThan)) => out.push('<'),
                &Token::DoubleQuoted(ref pos) => {
                    out.push('"');
                    out.push_str(self.sql.buffer_content(pos));
                    out.push('"');
                },
                &Token::SingleQuoted(ref pos) => {
                    out.push('\'');
                    out.push_str(self.sql.buffer_content(pos));
                    out.push('\'');
                },
                &Token::Backticked(ref pos) => {
                    out.push('`');
                    out.push_str(self.sql.buffer_content(pos));
                    out.push('`');
                },
                &Token::Numeric(ref pos) => {
                    out.push_str(self.sql.buffer_content(pos));
                },
                &Token::Space => out.push(' '),
                &Token::Newline => out.push('\n'),
                &Token::Placeholder => out.push('?'),
                &Token::Terminator => out.push(';')
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_write_single_quoted() {
        let sql = "SELECT `table`.* FROM `table` WHERE `id` = 'secret';";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_double_quoted() {
        let sql = "SELECT \"table\".* FROM \"table\" WHERE \"id\" = 'secret';";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_numeric() {
        let sql = "SELECT \"table\".* FROM \"table\" WHERE \"id\" = 1;";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_keywords() {
        let sql = "SELECT FROM WHERE AND IN OTHER";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_operators() {
        let sql = ". * <=> >= <= => => <> != = > <";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_newline() {
        let sql = "SELECT \"table\".*\nFROM \"table\" WHERE \"id\" = 1;";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_newline_in_string() {
        let sql = "SELECT \"table\".*\nFROM \"table\" WHERE \"i\nd\" = 1;";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_quote_in_quote() {
        let sql = "SELECT \"table\".*\nFROM \"table\" WHERE \"i\\\"d\" = 1;";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    mod helpers {
        pub fn lex_and_write(sql: String) -> String {
            super::super::super::write(super::super::super::lex(sql))
        }
    }
}
