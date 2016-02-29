use super::{Sql,Operator,ComparisonOperator,BufferPosition,Token,Keyword};

#[derive(Clone)]
pub struct SqlLexer {
    buf: String,
    len: usize,
    pos: usize
}

impl SqlLexer {
    pub fn new(buf: String) -> SqlLexer {
        let len = buf.len();
        SqlLexer {
            buf: buf,
            len: len,
            pos: 0
        }
    }

    fn find_until_char(&self, character: char) -> usize {
        let mut end = self.pos + 1;
        loop {
            if end >= self.len || self.buf.char_at(end) == character {
                break
            }
            end += 1;
        }
        end
    }

    fn find_until_char_with_possible_escaping(&self, character: char) -> usize {
        let mut end = self.pos + 1;
        let mut escape_char_count = 0;
        loop {
            if end >= self.len {
                break
            }
            let c = self.buf.char_at(end);
            if c == character && escape_char_count % 2 == 0  {
                break
            } else if c == '\\' {
                escape_char_count += 1;
            } else {
                escape_char_count = 0;
            }
            end += 1;
        }
        end
    }

    fn find_numeric_end(&self) -> usize {
        let mut end = self.pos + 1;
        loop {
            if end >= self.len {
                break
            }
            match self.buf.char_at(end) {
                '.' => end += 1,
                c if c.is_numeric() => end += 1,
                _ => break
            }
        }
        end
    }

    pub fn lex(mut self) -> Sql {
        let mut tokens = Vec::new();

        loop {
            if self.pos >= self.len {
                break
            }

            let token = match self.buf.char_at(self.pos) {
                '`' => {
                    let start = self.pos + 1;
                    let end = self.find_until_char('`');
                    self.pos = end + 1;
                    Token::Backticked(BufferPosition::new(start, end))
                },
                '\'' => {
                    let start = self.pos + 1;
                    let end = self.find_until_char_with_possible_escaping('\'');
                    self.pos = end + 1;
                    Token::SingleQuoted(BufferPosition::new(start, end))
                },
                '"' => {
                    let start = self.pos + 1;
                    let end = self.find_until_char_with_possible_escaping('"');
                    self.pos = end + 1;
                    Token::DoubleQuoted(BufferPosition::new(start, end))
                },
                '.' => {
                    self.pos += 1;
                    Token::Operator(Operator::Dot)
                },
                ',' => {
                    self.pos += 1;
                    Token::Operator(Operator::Comma)
                },
                '(' => {
                    self.pos += 1;
                    Token::Operator(Operator::ParentheseOpen)
                },
                ')' => {
                    self.pos += 1;
                    Token::Operator(Operator::ParentheseClose)
                },
                ':' => {
                    self.pos += 1;
                    Token::Operator(Operator::Colon)
                },
                '*' => {
                    self.pos += 1;
                    Token::Operator(Operator::Multiply)
                },
                '=' | '!' | '>' | '<' => {
                    let start = self.pos;
                    let end = self.find_until_char(' ');
                    self.pos = end;
                    match &self.buf[start..end] {
                        "<=>" => Token::Operator(Operator::Comparison(ComparisonOperator::NullSafeEqual)),
                        ">=" => Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)),
                        "<=" => Token::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)),
                        "=>" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrGreaterThan)),
                        "=<" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrLessThan)),
                        "<>" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualWithArrows)),
                        "!=" => Token::Operator(Operator::Comparison(ComparisonOperator::NotEqual)),
                        "=" => Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
                        ">" => Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)),
                        "<" => Token::Operator(Operator::Comparison(ComparisonOperator::LessThan)),
                        _ => break
                    }
                },
                ' ' => {
                    self.pos += 1;
                    Token::Space
                },
                '\n' | '\r' => {
                    self.pos += 1;
                    Token::Newline
                },
                ';' => {
                    self.pos += 1;
                    Token::Terminator
                },
                '?' => {
                    self.pos += 1;
                    Token::Placeholder
                },
                '$' => {
                    let start = self.pos;
                    let end = self.find_until_char(' ');
                    self.pos = end;
                    Token::NumberedPlaceholder(BufferPosition::new(start, end))
                },
                c if c.is_alphabetic() => {
                    let start = self.pos;
                    let end = self.find_until_char(' ');
                    self.pos = end;
                    let keyword: Option<Keyword> = match &self.buf[start..end] {
                        "SELECT" => Some(Keyword::Select),
                        "select" => Some(Keyword::Select),
                        "FROM" => Some(Keyword::From),
                        "from" => Some(Keyword::From),
                        "WHERE" => Some(Keyword::Where),
                        "where" => Some(Keyword::Where),
                        "AND" => Some(Keyword::And),
                        "and" => Some(Keyword::And),
                        "IN" => Some(Keyword::In),
                        "in" => Some(Keyword::In),
                        "UPDATE" => Some(Keyword::Update),
                        "update" => Some(Keyword::Update),
                        "SET" => Some(Keyword::Set),
                        "set" => Some(Keyword::Set),
                        "INSERT" => Some(Keyword::Insert),
                        "insert" => Some(Keyword::Insert),
                        "INTO" => Some(Keyword::Into),
                        "into" => Some(Keyword::Into),
                        "VALUES" => Some(Keyword::Values),
                        "values" => Some(Keyword::Values),
                        "INNER" => Some(Keyword::Inner),
                        "inner" => Some(Keyword::Inner),
                        "JOIN" => Some(Keyword::Join),
                        "join" => Some(Keyword::Join),
                        "ON" => Some(Keyword::On),
                        "on" => Some(Keyword::On),
                        _ => None
                    };
                    match keyword {
                        Some(k) => Token::Keyword(k),
                        None => Token::Keyword(Keyword::Other(BufferPosition::new(start, end)))
                    }
                },
                c if c.is_numeric() => {
                    let start = self.pos;
                    let end = self.find_numeric_end();
                    self.pos = end;
                    Token::Numeric(BufferPosition::new(start, end))
                },
                _ => break
            };

            tokens.push(token);
        }

        Sql {
            buf: self.buf,
            tokens: tokens
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SqlLexer;
    use super::super::{Token,Operator,ComparisonOperator,BufferPosition,Keyword};

    #[test]
    fn test_single_quoted_query() {
        let sql = "SELECT `table`.* FROM `table` WHERE `id` = 'secret' and `other` = 'something';".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Backticked(BufferPosition::new(8, 13)),
            Token::Operator(Operator::Dot),
            Token::Operator(Operator::Multiply),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Backticked(BufferPosition::new(23, 28)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Backticked(BufferPosition::new(37, 39)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::SingleQuoted(BufferPosition::new(44, 50)),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::Backticked(BufferPosition::new(57, 62)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::SingleQuoted(BufferPosition::new(67, 76)),
            Token::Terminator
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_double_quoted_and_numeric_query() {
        let sql = "SELECT \"table\".* FROM \"table\" WHERE \"id\" = 18 AND \"number\" = 18.0;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(8, 13)),
            Token::Operator(Operator::Dot),
            Token::Operator(Operator::Multiply),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(23, 28)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(37, 39)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Numeric(BufferPosition::new(43, 45)),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(51, 57)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Numeric(BufferPosition::new(61, 65)),
            Token::Terminator
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_in_query() {
        let sql = "SELECT * FROM \"table\" WHERE \"id\" IN (1,2,3);".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Operator(Operator::Multiply),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(15, 20)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(29, 31)),
            Token::Space,
            Token::Keyword(Keyword::In),
            Token::Space,
            Token::Operator(Operator::ParentheseOpen),
            Token::Numeric(BufferPosition::new(37, 38)),
            Token::Operator(Operator::Comma),
            Token::Numeric(BufferPosition::new(39, 40)),
            Token::Operator(Operator::Comma),
            Token::Numeric(BufferPosition::new(41, 42)),
            Token::Operator(Operator::ParentheseClose),
            Token::Terminator
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_comparison_operators() {
        let sql = "<=> >= <= => =< <> != = > <".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Comparison(ComparisonOperator::NullSafeEqual)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrGreaterThan)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrLessThan)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::EqualWithArrows)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::NotEqual)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::LessThan))
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_known_keywords() {
        let sql = "SELECT FROM WHERE AND IN UPDATE SET INSERT INTO VALUES INNER JOIN ON".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::Keyword(Keyword::In),
            Token::Space,
            Token::Keyword(Keyword::Update),
            Token::Space,
            Token::Keyword(Keyword::Set),
            Token::Space,
            Token::Keyword(Keyword::Insert),
            Token::Space,
            Token::Keyword(Keyword::Into),
            Token::Space,
            Token::Keyword(Keyword::Values),
            Token::Space,
            Token::Keyword(Keyword::Inner),
            Token::Space,
            Token::Keyword(Keyword::Join),
            Token::Space,
            Token::Keyword(Keyword::On)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_known_keywords_lowercase() {
        let sql = "select from where and in update set insert into values inner join on".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::Keyword(Keyword::In),
            Token::Space,
            Token::Keyword(Keyword::Update),
            Token::Space,
            Token::Keyword(Keyword::Set),
            Token::Space,
            Token::Keyword(Keyword::Insert),
            Token::Space,
            Token::Keyword(Keyword::Into),
            Token::Space,
            Token::Keyword(Keyword::Values),
            Token::Space,
            Token::Keyword(Keyword::Inner),
            Token::Space,
            Token::Keyword(Keyword::Join),
            Token::Space,
            Token::Keyword(Keyword::On)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_obscure_keyword() {
        let sql = "OBSCURE FROM".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Other(BufferPosition::new(0, 7))),
            Token::Space,
            Token::Keyword(Keyword::From)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_single_quoted() {
        let sql = "'val\\'ue' FROM 'sec\nret\\\\';".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::SingleQuoted(BufferPosition::new(1, 8)),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::SingleQuoted(BufferPosition::new(16, 25)),
            Token::Terminator
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_double_quoted() {
        let sql = "\"val\\\"ue\" FROM \"sec\nret\\\\\";".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::DoubleQuoted(BufferPosition::new(1, 8)),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferPosition::new(16, 25)),
            Token::Terminator
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_placeholders() {
        let sql = "? $1 $2 $23".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Placeholder,
            Token::Space,
            Token::NumberedPlaceholder(BufferPosition::new(2, 4)),
            Token::Space,
            Token::NumberedPlaceholder(BufferPosition::new(5, 7)),
            Token::Space,
            Token::NumberedPlaceholder(BufferPosition::new(8, 11))
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }
}
