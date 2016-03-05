use super::{Sql,Operator,ArithmeticOperator,BitwiseOperator,ComparisonOperator,LogicalOperator,BufferSlice,Token,Keyword};

#[derive(Clone,PartialEq)]
enum State {
    Default,
    PastSelect,
    PastFrom
}

#[derive(Clone)]
pub struct SqlLexer {
    state: State,
    buf: String,
    len: usize,
    pos: usize
}

impl SqlLexer {
    pub fn new(buf: String) -> SqlLexer {
        let len = buf.len();
        SqlLexer {
            state: State::Default,
            buf: buf,
            len: len,
            pos: 0
        }
    }

    fn find_until<F>(&self, at_end_function: F) -> usize where F: Fn(char) -> bool {
        let mut end = self.pos + 1;
        loop {
            if end >= self.len || at_end_function(self.buf.char_at(end)) {
                break
            }
            end += 1;
        }
        end
    }

    fn find_until_delimiter_with_possible_escaping(&self, character: char) -> usize {
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

    pub fn lex(mut self) -> Sql {
        let mut tokens = Vec::new();

        loop {
            if self.pos >= self.len {
                break
            }

            let token = match self.buf.char_at(self.pos) {
                // Back quoted
                '`' => {
                    let start = self.pos + 1;
                    let end = self.find_until(|c| c == '`');
                    self.pos = end + 1;
                    Token::Backticked(BufferSlice::new(start, end))
                },
                // Single quoted
                '\'' => {
                    let start = self.pos + 1;
                    let end = self.find_until_delimiter_with_possible_escaping('\'');
                    self.pos = end + 1;
                    Token::SingleQuoted(BufferSlice::new(start, end))
                },
                // Double quoted
                '"' => {
                    let start = self.pos + 1;
                    let end = self.find_until_delimiter_with_possible_escaping('"');
                    self.pos = end + 1;
                    Token::DoubleQuoted(BufferSlice::new(start, end))
                },
                // Pound comment
                '#' => {
                    let start = self.pos;
                    let end = self.find_until(|c| c == '\n' || c == '\r');
                    self.pos = end;
                    Token::Comment(BufferSlice::new(start, end))
                },
                // Double dash comment
                '-' if self.pos + 1 < self.len && self.buf.char_at(self.pos + 1) == '-' => {
                    let start = self.pos;
                    let end = self.find_until(|c| c == '\n' || c == '\r');
                    self.pos = end;
                    Token::Comment(BufferSlice::new(start, end))
                },
                // Multi line comment
                '/' if self.pos + 1 < self.len && self.buf.char_at(self.pos + 1) == '*' => {
                    let start = self.pos;
                    let mut end = self.pos + 2;
                    loop {
                        if end >= self.len || (self.buf.char_at(end) == '/' && self.buf.char_at(end - 1) == '*') {
                            break
                        }
                        end += 1;
                    }
                    end += 1;
                    self.pos = end;
                    Token::Comment(BufferSlice::new(start, end))
                },
                // Generic tokens
                ' ' => {
                    self.pos += 1;
                    Token::Space
                },
                '\n' | '\r' => {
                    self.pos += 1;
                    Token::Newline
                },
                '.' => {
                    self.pos += 1;
                    Token::Dot
                },
                ',' => {
                    self.pos += 1;
                    Token::Comma
                },
                '(' => {
                    self.pos += 1;
                    Token::ParentheseOpen
                },
                ')' => {
                    self.pos += 1;
                    Token::ParentheseClose
                },
                ':' => {
                    self.pos += 1;
                    Token::Colon
                },
                ';' => {
                    self.pos += 1;
                    Token::Semicolon
                },
                '?' => {
                    self.pos += 1;
                    Token::Placeholder
                },
                '$' => {
                    let start = self.pos;
                    let end = self.find_until(|c| !c.is_numeric() );
                    self.pos = end;
                    Token::NumberedPlaceholder(BufferSlice::new(start, end))
                },
                // Arithmetic operators
                '*' => {
                    self.pos += 1;
                    match self.state {
                        State::PastSelect => Token::Wildcard,
                        _ => Token::Operator(Operator::Arithmetic(ArithmeticOperator::Multiply))
                    }
                },
                '/' => {
                    self.pos += 1;
                    Token::Operator(Operator::Arithmetic(ArithmeticOperator::Divide))
                },
                '%' => {
                    self.pos += 1;
                    Token::Operator(Operator::Arithmetic(ArithmeticOperator::Modulo))
                },
                '+' => {
                    self.pos += 1;
                    Token::Operator(Operator::Arithmetic(ArithmeticOperator::Plus))
                },
                '-' => {
                    self.pos += 1;
                    Token::Operator(Operator::Arithmetic(ArithmeticOperator::Minus))
                },
                // Comparison and bitwise operators
                '=' | '!' | '>' | '<' | '&' | '|' => {
                    let start = self.pos;
                    let end = self.find_until(|c| {
                        match c {
                            '=' | '!' | '>' | '<' => false,
                            _ => true
                        }
                    });
                    self.pos = end;
                    match &self.buf[start..end] {
                        // Comparison
                        "<=>" => Token::Operator(Operator::Comparison(ComparisonOperator::NullSafeEqual)),
                        ">=" => Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)),
                        "<=" => Token::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)),
                        "=>" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrGreaterThan)),
                        "=<" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualOrLessThan)),
                        "<>" => Token::Operator(Operator::Comparison(ComparisonOperator::EqualWithArrows)),
                        "!=" => Token::Operator(Operator::Comparison(ComparisonOperator::NotEqual)),
                        "==" => Token::Operator(Operator::Comparison(ComparisonOperator::Equal2)),
                        "=" => Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
                        ">" => Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)),
                        "<" => Token::Operator(Operator::Comparison(ComparisonOperator::LessThan)),
                        // Bitwise
                        "<<" => Token::Operator(Operator::Bitwise(BitwiseOperator::LeftShift)),
                        ">>" => Token::Operator(Operator::Bitwise(BitwiseOperator::RightShift)),
                        "&" => Token::Operator(Operator::Bitwise(BitwiseOperator::And)),
                        "|" => Token::Operator(Operator::Bitwise(BitwiseOperator::Or)),
                        _ => break
                    }
                },
                // Logical operators and keywords
                c if c.is_alphabetic() => {
                    let start = self.pos;
                    let end = self.find_until(|c| {
                        match c {
                            '_' => false,
                            '-' => false,
                            c if c.is_alphabetic() => false,
                            c if c.is_numeric() => false,
                            _ => true
                        }
                    });
                    self.pos = end;
                    match &self.buf[start..end] {
                        // Keywords
                        "SELECT" | "select" => {
                            self.state = State::PastSelect;
                            Token::Keyword(Keyword::Select)
                        },
                        "FROM" | "from"=> {
                            self.state = State::PastFrom;
                            Token::Keyword(Keyword::From)
                        },
                        "WHERE" | "where" => Token::Keyword(Keyword::Where),
                        "AND" | "and" => Token::Keyword(Keyword::And),
                        "OR" | "or" => Token::Keyword(Keyword::Or),
                        "UPDATE" | "update" => Token::Keyword(Keyword::Update),
                        "SET" | "set" => Token::Keyword(Keyword::Set),
                        "INSERT" | "insert" => Token::Keyword(Keyword::Insert),
                        "INTO" | "into" => Token::Keyword(Keyword::Into),
                        "VALUES" | "values" => Token::Keyword(Keyword::Values),
                        "INNER" | "inner" => Token::Keyword(Keyword::Inner),
                        "JOIN" | "join" => Token::Keyword(Keyword::Join),
                        "ON" | "on" => Token::Keyword(Keyword::On),
                        "LIMIT" | "limit" => Token::Keyword(Keyword::Limit),
                        "OFFSET" | "offset" => Token::Keyword(Keyword::Offset),
                        "BETWEEN" | "between" => Token::Keyword(Keyword::Between),
                        // Logical operators
                        "IN" | "in" => Token::Operator(Operator::Logical(LogicalOperator::In)),
                        "NOT" | "not" => Token::Operator(Operator::Logical(LogicalOperator::Not)),
                        "LIKE" | "like" => Token::Operator(Operator::Logical(LogicalOperator::Like)),
                        "RLIKE" | "rlike" => Token::Operator(Operator::Logical(LogicalOperator::Rlike)),
                        "GLOB" | "glob" => Token::Operator(Operator::Logical(LogicalOperator::Glob)),
                        "MATCH" | "match" => Token::Operator(Operator::Logical(LogicalOperator::Match)),
                        "REGEXP" | "regexp" => Token::Operator(Operator::Logical(LogicalOperator::Regexp)),
                        // Other keyword
                        _ => Token::Keyword(Keyword::Other(BufferSlice::new(start, end)))
                    }
                },
                // Numeric
                c if c.is_numeric() => {
                    let start = self.pos;
                    let end = self.find_until(|c| {
                        match c {
                            '.' => false,
                            c if c.is_numeric() => false,
                            _ => true
                        }
                    });
                    self.pos = end;
                    Token::Numeric(BufferSlice::new(start, end))
                },
                // Unknown
                c => {
                    self.pos += 1;
                    Token::Unknown(c)
                }
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
    use super::super::{Token,Operator,ArithmeticOperator,BitwiseOperator,ComparisonOperator,LogicalOperator,BufferSlice,Keyword};

    #[test]
    fn test_single_quoted_query() {
        let sql = "SELECT `table`.* FROM `table` WHERE `id` = 'secret' and `other` = 'something';".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Backticked(BufferSlice::new(8, 13)),
            Token::Dot,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Backticked(BufferSlice::new(23, 28)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Backticked(BufferSlice::new(37, 39)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::SingleQuoted(BufferSlice::new(44, 50)),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::Backticked(BufferSlice::new(57, 62)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::SingleQuoted(BufferSlice::new(67, 76)),
            Token::Semicolon
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
            Token::DoubleQuoted(BufferSlice::new(8, 13)),
            Token::Dot,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(23, 28)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(37, 39)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Numeric(BufferSlice::new(43, 45)),
            Token::Space,
            Token::Keyword(Keyword::And),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(51, 57)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Numeric(BufferSlice::new(61, 65)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_double_quoted_and_numeric_query_no_whitespace() {
        let sql = "SELECT\"table\".*FROM\"table\"WHERE\"id\"=18AND\"number\"=18.0;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::DoubleQuoted(BufferSlice::new(7, 12)),
            Token::Dot,
            Token::Wildcard,
            Token::Keyword(Keyword::From),
            Token::DoubleQuoted(BufferSlice::new(20, 25)),
            Token::Keyword(Keyword::Where),
            Token::DoubleQuoted(BufferSlice::new(32, 34)),
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Numeric(BufferSlice::new(36, 38)),
            Token::Keyword(Keyword::And),
            Token::DoubleQuoted(BufferSlice::new(42, 48)),
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Numeric(BufferSlice::new(50, 54)),
            Token::Semicolon
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
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(15, 20)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(29, 31)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::In)),
            Token::Space,
            Token::ParentheseOpen,
            Token::Numeric(BufferSlice::new(37, 38)),
            Token::Comma,
            Token::Numeric(BufferSlice::new(39, 40)),
            Token::Comma,
            Token::Numeric(BufferSlice::new(41, 42)),
            Token::ParentheseClose,
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_arithmetic_operators() {
        // See if we properly distinguish between wildcard and multiplier
        let sql = "SELECT * FROM WHERE * / % + -;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Operator(Operator::Arithmetic(ArithmeticOperator::Multiply)),
            Token::Space,
            Token::Operator(Operator::Arithmetic(ArithmeticOperator::Divide)),
            Token::Space,
            Token::Operator(Operator::Arithmetic(ArithmeticOperator::Modulo)),
            Token::Space,
            Token::Operator(Operator::Arithmetic(ArithmeticOperator::Plus)),
            Token::Space,
            Token::Operator(Operator::Arithmetic(ArithmeticOperator::Minus)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_logical_operators() {
        let sql = "IN NOT LIKE RLIKE GLOB MATCH REGEXP".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Logical(LogicalOperator::In)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Not)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Like)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Rlike)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Glob)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Match)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Regexp)),
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_logical_operators_lowercase() {
        let sql = "in not like rlike glob match regexp".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Logical(LogicalOperator::In)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Not)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Like)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Rlike)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Glob)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Match)),
            Token::Space,
            Token::Operator(Operator::Logical(LogicalOperator::Regexp)),
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_comparison_operators() {
        let sql = "= == <=> >= <= => =< <> != > <;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal2)),
            Token::Space,
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
            Token::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::LessThan)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_bitwise_operators() {
        let sql = "<< >> & |;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Bitwise(BitwiseOperator::LeftShift)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::RightShift)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::And)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::Or)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_known_keywords() {
        let sql = "SELECT FROM WHERE AND OR UPDATE SET INSERT INTO VALUES INNER JOIN ON LIMIT OFFSET BETWEEN;".to_string();
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
            Token::Keyword(Keyword::Or),
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
            Token::Keyword(Keyword::On),
            Token::Space,
            Token::Keyword(Keyword::Limit),
            Token::Space,
            Token::Keyword(Keyword::Offset),
            Token::Space,
            Token::Keyword(Keyword::Between),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_known_keywords_lowercase() {
        let sql = "select from where and or update set insert into values inner join on limit offset between;".to_string();
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
            Token::Keyword(Keyword::Or),
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
            Token::Keyword(Keyword::On),
            Token::Space,
            Token::Keyword(Keyword::Limit),
            Token::Space,
            Token::Keyword(Keyword::Offset),
            Token::Space,
            Token::Keyword(Keyword::Between),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_obscure_keyword() {
        let sql = "OBSCURE FROM;".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Other(BufferSlice::new(0, 7))),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_single_quoted() {
        let sql = "'val\\'ue' FROM 'sec\nret\\\\';".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::SingleQuoted(BufferSlice::new(1, 8)),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::SingleQuoted(BufferSlice::new(16, 25)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_double_quoted() {
        let sql = "\"val\\\"ue\" FROM \"sec\nret\\\\\";".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::DoubleQuoted(BufferSlice::new(1, 8)),
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::DoubleQuoted(BufferSlice::new(16, 25)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_placeholders() {
        let sql = "? $1 $2 $23;".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Placeholder,
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(2, 4)),
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(5, 7)),
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(8, 11)),
            Token::Semicolon
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_unknown() {
        let sql = "~ ^".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Unknown('~'),
            Token::Space,
            Token::Unknown('^'),
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_comment_pound() {
        let sql = "SELECT * FROM table # This is a comment\n SELECT".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Other(BufferSlice::new(14, 19))),
            Token::Space,
            Token::Comment(BufferSlice::new(20, 39)),
            Token::Newline,
            Token::Space,
            Token::Keyword(Keyword::Select)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_comment_double_dash() {
        let sql = "SELECT * FROM table -- This is a comment\n SELECT".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Other(BufferSlice::new(14, 19))),
            Token::Space,
            Token::Comment(BufferSlice::new(20, 40)),
            Token::Newline,
            Token::Space,
            Token::Keyword(Keyword::Select)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_comment_multi_line() {
        let sql = "SELECT * FROM table /* This is a comment */ SELECT".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Keyword(Keyword::Other(BufferSlice::new(14, 19))),
            Token::Space,
            Token::Comment(BufferSlice::new(20, 43)),
            Token::Space,
            Token::Keyword(Keyword::Select)
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }
}
