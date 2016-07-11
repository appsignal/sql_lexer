use super::{Keyword,Token,Operator,ArithmeticOperator,ComparisonOperator,LogicalOperator,BitwiseOperator,Sql};

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
                // Arithmetic operator
                &Token::Operator(Operator::Arithmetic(ArithmeticOperator::Multiply)) => out.push('*'),
                &Token::Operator(Operator::Arithmetic(ArithmeticOperator::Divide)) => out.push('/'),
                &Token::Operator(Operator::Arithmetic(ArithmeticOperator::Modulo)) => out.push('%'),
                &Token::Operator(Operator::Arithmetic(ArithmeticOperator::Plus)) => out.push('+'),
                &Token::Operator(Operator::Arithmetic(ArithmeticOperator::Minus)) => out.push('-'),
                // Logical operator
                &Token::Operator(Operator::Logical(LogicalOperator::In)) => out.push_str("IN"),
                &Token::Operator(Operator::Logical(LogicalOperator::Not)) => out.push_str("NOT"),
                &Token::Operator(Operator::Logical(LogicalOperator::Like)) => out.push_str("LIKE"),
                &Token::Operator(Operator::Logical(LogicalOperator::Rlike)) => out.push_str("RLIKE"),
                &Token::Operator(Operator::Logical(LogicalOperator::Glob)) => out.push_str("GLOB"),
                &Token::Operator(Operator::Logical(LogicalOperator::Match)) => out.push_str("MATCH"),
                &Token::Operator(Operator::Logical(LogicalOperator::Regexp)) => out.push_str("REGEXP"),
                // Comparison operator
                &Token::Operator(Operator::Comparison(ComparisonOperator::Equal)) => out.push('='),
                &Token::Operator(Operator::Comparison(ComparisonOperator::Equal2)) => out.push_str("=="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::NullSafeEqual)) => out.push_str("<=>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)) => out.push_str(">="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)) => out.push_str("<="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrGreaterThan)) => out.push_str("=>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrLessThan)) => out.push_str("<="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::EqualWithArrows)) => out.push_str("<>"),
                &Token::Operator(Operator::Comparison(ComparisonOperator::NotEqual)) => out.push_str("!="),
                &Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)) => out.push('>'),
                &Token::Operator(Operator::Comparison(ComparisonOperator::LessThan)) => out.push('<'),
                // Bitwise operator
                &Token::Operator(Operator::Bitwise(BitwiseOperator::LeftShift)) => out.push_str("<<"),
                &Token::Operator(Operator::Bitwise(BitwiseOperator::RightShift)) => out.push_str(">>"),
                &Token::Operator(Operator::Bitwise(BitwiseOperator::And)) => out.push('&'),
                &Token::Operator(Operator::Bitwise(BitwiseOperator::Or)) => out.push('|'),
                // Keywords
                &Token::Keyword(Keyword::Select) => out.push_str("SELECT"),
                &Token::Keyword(Keyword::From) => out.push_str("FROM"),
                &Token::Keyword(Keyword::Where) => out.push_str("WHERE"),
                &Token::Keyword(Keyword::Update) => out.push_str("UPDATE"),
                &Token::Keyword(Keyword::Set) => out.push_str("SET"),
                &Token::Keyword(Keyword::Insert) => out.push_str("INSERT"),
                &Token::Keyword(Keyword::Into) => out.push_str("INTO"),
                &Token::Keyword(Keyword::Values) => out.push_str("VALUES"),
                &Token::Keyword(Keyword::Inner) => out.push_str("INNER"),
                &Token::Keyword(Keyword::Join) => out.push_str("JOIN"),
                &Token::Keyword(Keyword::On) => out.push_str("ON"),
                &Token::Keyword(Keyword::And) => out.push_str("AND"),
                &Token::Keyword(Keyword::Or) => out.push_str("OR"),
                &Token::Keyword(Keyword::Limit) => out.push_str("LIMIT"),
                &Token::Keyword(Keyword::Offset) => out.push_str("OFFSET"),
                &Token::Keyword(Keyword::Between) => out.push_str("BETWEEN"),
                &Token::Keyword(Keyword::Array) => out.push_str("ARRAY"),
                &Token::Keyword(Keyword::Other(ref slice)) => {
                    out.push_str(self.sql.buffer_content(slice));
                },
                // Backticked
                &Token::Backticked(ref slice) => {
                    out.push('`');
                    out.push_str(self.sql.buffer_content(slice));
                    out.push('`');
                },
                // Double quoted
                &Token::DoubleQuoted(ref slice) => {
                    out.push('"');
                    out.push_str(self.sql.buffer_content(slice));
                    out.push('"');
                },
                // Single quoted
                &Token::SingleQuoted(ref slice) => {
                    out.push('\'');
                    out.push_str(self.sql.buffer_content(slice));
                    out.push('\'');
                },
                // Numeric
                &Token::Numeric(ref slice) => {
                    out.push_str(self.sql.buffer_content(slice));
                },
                // Comment
                &Token::Comment(ref slice) => {
                    out.push_str(self.sql.buffer_content(slice));
                },
                // Generic tokens
                &Token::Space => out.push(' '),
                &Token::Newline => out.push('\n'),
                &Token::Dot => out.push('.'),
                &Token::Comma => out.push(','),
                &Token::Wildcard => out.push('*'),
                &Token::ParentheseOpen => out.push('('),
                &Token::ParentheseClose => out.push(')'),
                &Token::SquareBracketOpen => out.push('['),
                &Token::SquareBracketClose => out.push(']'),
                &Token::Colon => out.push(':'),
                &Token::Semicolon => out.push(';'),
                &Token::Placeholder => out.push('?'),
                &Token::Null => out.push_str("NULL"),
                &Token::NumberedPlaceholder(ref slice) => {
                    out.push_str(self.sql.buffer_content(slice));
                },
                &Token::Unknown(c) => {
                    out.push(c);
                }
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
    fn test_tokens() {
        let sql = " . , * ( ) [ ] : ; ?";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_keywords() {
        let sql = "SELECT FROM WHERE AND OR UPDATE SET INSERT INTO VALUES INNER JOIN ON OTHER LIMIT OFFSET BETWEEN ARRAY";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_arithmetic_operators() {
        let sql = "* / % + -";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_logical_operators() {
        let sql = "IN NOT LIKE RLIKE GLOB MATCH REGEXP";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_comparison_operators() {
        let sql = "= == <=> >= <= => => <> != > <";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_bitwise_operators() {
        let sql = "<< >> & |";
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

    #[test]
    fn test_write_placeholders() {
        let sql = "? $1 $2 $23";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_null() {
        let sql = "NULL";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_write_unknown() {
        let sql = "~ #";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_comment_pound() {
        let sql = "SELECT * FROM table # This is a comment";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_comment_double_dash() {
        let sql = "SELECT * FROM table -- This is a comment";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_comment_multi_line() {
        let sql = "SELECT * FROM table /* This is a comment */";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    #[test]
    fn test_empty() {
        let sql = "";
        let written = helpers::lex_and_write(sql.to_string());

        assert_eq!(written, sql);
    }

    mod helpers {
        pub fn lex_and_write(sql: String) -> String {
            super::super::super::write(super::super::super::lex(sql))
        }
    }
}
