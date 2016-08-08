use super::{LiteralValueTypeIndicator,Sql,Operator,ArithmeticOperator,BitwiseOperator,ComparisonOperator,LogicalOperator,BufferSlice,Token,Keyword};

#[derive(Clone,PartialEq)]
enum State {
    Default,
    PastSelect,
    PastFrom
}

#[derive(Clone)]
pub struct SqlLexer {
    state:        State,
    buf:          String,
    char_indices: Vec<(usize, char)>,
    len:          usize,
    pos:          usize
}

impl SqlLexer {
    pub fn new(buf: String) -> SqlLexer {
        let char_indices: Vec<(usize, char)> = buf.char_indices().collect();
        let len = char_indices.len();
        SqlLexer {
            state:        State::Default,
            buf:          buf,
            char_indices: char_indices,
            len:          len,
            pos:           0
        }
    }

    fn char_at(&self, pos: usize) -> char {
        self.char_indices[pos].1
    }

    fn scan_until<F>(&mut self, mut current_byte_offset: usize, at_end_function: F) -> usize where F: Fn(&SqlLexer, char) -> bool {
        self.pos += 1;
        loop {
            if self.pos >= self.len {
                // We're at the end, we want to include the last character
                if self.pos > 0 {
                    current_byte_offset += self.char_indices[self.pos - 1].1.len_utf8();
                }
                break
            }
            let indice = self.char_indices[self.pos];
            current_byte_offset = indice.0;
            if at_end_function(&self, indice.1) {
                break
            }
            self.pos += 1;
        }
        current_byte_offset
    }

    fn scan_for_delimiter_with_possible_escaping(&mut self, mut current_byte_offset: usize, delimiter: char) -> usize {
        let mut escape_char_count = 0;
        self.pos += 1;
        loop {
            if self.pos >= self.len {
                // We're at the end, we want to include the last character
                if self.pos > 0 {
                    current_byte_offset += self.char_indices[self.pos - 1].1.len_utf8();
                }
                break
            }
            let indice = self.char_indices[self.pos];
            current_byte_offset = indice.0;
            if indice.1 == delimiter && escape_char_count % 2 == 0  {
                self.pos += 1;
                break
            } else if indice.1 == '\\' {
                escape_char_count += 1;
            } else {
                escape_char_count = 0;
            }
            self.pos += 1;
        }
        current_byte_offset
    }

    pub fn lex(mut self) -> Sql {
        let mut tokens = Vec::new();

        loop {
            if self.pos >= self.len {
                break
            }

            let (current_byte_offset, current_char) = self.char_indices[self.pos];

            let token = match current_char {
                // Back quoted
                '`' => {
                    let end_byte_offset = self.scan_for_delimiter_with_possible_escaping(current_byte_offset, '`');
                    Token::Backticked(BufferSlice::new(current_byte_offset + 1, end_byte_offset))
                },
                // Single quoted
                '\'' => {
                    let end_byte_offset = self.scan_for_delimiter_with_possible_escaping(current_byte_offset, '\'');
                    Token::SingleQuoted(BufferSlice::new(current_byte_offset + 1, end_byte_offset))
                },
                // Double quoted
                '"' => {
                    let end_byte_offset = self.scan_for_delimiter_with_possible_escaping(current_byte_offset, '"');
                    Token::DoubleQuoted(BufferSlice::new(current_byte_offset + 1, end_byte_offset))
                },
                // Pound comment
                '#' => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| c == '\n' || c == '\r');
                    Token::Comment(BufferSlice::new(current_byte_offset, end_byte_offset))
                },
                // Double dash comment
                '-' if self.pos + 1 < self.len && self.char_at(self.pos + 1) == '-' => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| c == '\n' || c == '\r');
                    Token::Comment(BufferSlice::new(current_byte_offset, end_byte_offset))
                },
                // Multi line comment
                '/' if self.pos + 1 < self.len && self.char_at(self.pos + 1) == '*' => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |lexer, _| {
                        lexer.pos > 1 &&
                            lexer.char_at(lexer.pos -2) == '*' &&
                            lexer.char_at(lexer.pos -1) == '/'
                    });
                    Token::Comment(BufferSlice::new(current_byte_offset, end_byte_offset))
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
                '[' => {
                    self.pos += 1;
                    Token::SquareBracketOpen
                },
                ']' => {
                    self.pos += 1;
                    Token::SquareBracketClose
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
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| !c.is_numeric() );
                    Token::NumberedPlaceholder(BufferSlice::new(current_byte_offset, end_byte_offset))
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
                '-' if !(self.pos + 1 < self.len && self.char_at(self.pos +1).is_numeric()) => {
                    self.pos += 1;
                    Token::Operator(Operator::Arithmetic(ArithmeticOperator::Minus))
                },
                // Comparison and bitwise operators
                '=' | '!' | '>' | '<' | '&' | '|' => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| {
                        match c {
                            '=' | '!' | '>' | '<' => false,
                            _ => true
                        }
                    });
                    match &self.buf[current_byte_offset..end_byte_offset] {
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
                // Charset literal value type indicator
                '_' => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| {
                        match c {
                            c if c.is_alphabetic() => false,
                            c if c.is_numeric() => false,
                            _ => true
                        }
                    });
                    Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Charset(BufferSlice::new(current_byte_offset + 1, end_byte_offset)))
                },
                // Logical operators and keywords
                c if c.is_alphabetic() => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| {
                        match c {
                            '_' => false,
                            '-' => false,
                            c if c.is_alphabetic() => false,
                            c if c.is_numeric() => false,
                            _ => true
                        }
                    });
                    match &self.buf[current_byte_offset..end_byte_offset] {
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
                        "ARRAY" | "array" => Token::Keyword(Keyword::Array),
                        // Logical operators
                        "IN" | "in" => Token::Operator(Operator::Logical(LogicalOperator::In)),
                        "NOT" | "not" => Token::Operator(Operator::Logical(LogicalOperator::Not)),
                        "LIKE" | "like" => Token::Operator(Operator::Logical(LogicalOperator::Like)),
                        "RLIKE" | "rlike" => Token::Operator(Operator::Logical(LogicalOperator::Rlike)),
                        "GLOB" | "glob" => Token::Operator(Operator::Logical(LogicalOperator::Glob)),
                        "MATCH" | "match" => Token::Operator(Operator::Logical(LogicalOperator::Match)),
                        "REGEXP" | "regexp" => Token::Operator(Operator::Logical(LogicalOperator::Regexp)),
                        // Some of the literal value type indicators
                        "DATE" | "date" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Date),
                        "TIME" | "time" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Time),
                        "TIMESTAMP" | "timestamp" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Timestamp),
                        "X" | "x" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::X),
                        "B" | "b" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::B),
                        "N" | "n" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::N),
                        // Null
                        "NULL" | "null" => Token::Null,
                        // Other keyword
                        _ => Token::Keyword(Keyword::Other(BufferSlice::new(current_byte_offset, end_byte_offset)))
                    }
                },
                // Numeric
                c if c == '-' || c.is_numeric() => {
                    let end_byte_offset = self.scan_until(current_byte_offset, |_, c| {
                        match c {
                            '.' => false,
                            'x' | 'X' | 'b' | 'B' => false,
                            c if c.is_numeric() => false,
                            _ => true
                        }
                    });
                    match &self.buf[current_byte_offset..end_byte_offset] {
                        "0X" | "0x" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroX),
                        "0B" | "0b" => Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroB),
                        _ => Token::Numeric(BufferSlice::new(current_byte_offset, end_byte_offset))
                    }
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
    use super::super::{Token,LiteralValueTypeIndicator,Operator,ArithmeticOperator,BitwiseOperator,ComparisonOperator,LogicalOperator,BufferSlice,Keyword};

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
    fn test_array_query() {
        let sql = "SELECT * FROM \"table\" WHERE \"field\" = ARRAY['item_1','item_2','item_3']".to_string();
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
            Token::DoubleQuoted(BufferSlice::new(29, 34)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Keyword(Keyword::Array),
            Token::SquareBracketOpen,
            Token::SingleQuoted(BufferSlice::new(45, 51)),
            Token::Comma,
            Token::SingleQuoted(BufferSlice::new(54, 60)),
            Token::Comma,
            Token::SingleQuoted(BufferSlice::new(63, 69)),
            Token::SquareBracketClose
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
    fn test_numeric() {
        let sql = "1 1.0 -1 -1.0".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Numeric(BufferSlice::new(0, 1)),
            Token::Space,
            Token::Numeric(BufferSlice::new(2, 5)),
            Token::Space,
            Token::Numeric(BufferSlice::new(6, 8)),
            Token::Space,
            Token::Numeric(BufferSlice::new(9, 13))
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
    fn test_comparison_operators_end_of_line() {
        let sql = "= == <=> >= <= => =< <> != > <".to_string();
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
            Token::Operator(Operator::Comparison(ComparisonOperator::LessThan))
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
    fn test_bitwise_operator_end_of_line() {
        let sql = "<< >> & |".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Operator(Operator::Bitwise(BitwiseOperator::LeftShift)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::RightShift)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::And)),
            Token::Space,
            Token::Operator(Operator::Bitwise(BitwiseOperator::Or))
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
        let sql = "select from where and or update set insert into values inner join on limit offset between".to_string();
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
            Token::Keyword(Keyword::Between)
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
    fn test_obscure_keyword_end_of_line() {
        let sql = "OBSCURE".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::Keyword(Keyword::Other(BufferSlice::new(0, 7)))
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_literal_value_type_indicator_uppercase() {
        let sql = "DATE TIME TIMESTAMP X 0X B 0B N".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Date),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Time),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Timestamp),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::X),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroX),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::B),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroB),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::N),
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }


    #[test]
    fn test_literal_value_type_indicator_lowercase() {
        let sql = "date time timestamp x 0x b 0b n _utf8".to_string();
        let lexer = SqlLexer::new(sql);

        let expected = vec![
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Date),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Time),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Timestamp),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::X),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroX),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::B),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::ZeroB),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::N),
            Token::Space,
            Token::LiteralValueTypeIndicator(LiteralValueTypeIndicator::Charset(BufferSlice::new(33, 37)))
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
    fn test_quoted_missing_delimiter() {
        let sql = "\"val\\\"ue".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::DoubleQuoted(BufferSlice::new(1, 8))
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_multibyte_characters() {
        let sql = "\"hæld\" ; 'jæld' ; `tæld`".to_string();
        let lexer = SqlLexer::new(sql);

        // These are one byte longer than the number of characters since
        // æ is two bytes long.
        let expected = vec![
            Token::DoubleQuoted(BufferSlice::new(1, 6)),
            Token::Space,
            Token::Semicolon,
            Token::Space,
            Token::SingleQuoted(BufferSlice::new(11, 16)),
            Token::Space,
            Token::Semicolon,
            Token::Space,
            Token::Backticked(BufferSlice::new(21, 26))
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
    fn test_placeholders_end_of_line() {
        let sql = "? $1 $2 $23".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Placeholder,
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(2, 4)),
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(5, 7)),
            Token::Space,
            Token::NumberedPlaceholder(BufferSlice::new(8, 11))
        ];

        assert_eq!(lexer.lex().tokens, expected);
    }

    #[test]
    fn test_null() {
        let sql = "NULL null".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![
            Token::Null,
            Token::Space,
            Token::Null
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

    #[test]
    fn test_empty() {
        let sql = "".to_string();
        let lexer = SqlLexer::new(sql);
        let expected = vec![];

        assert_eq!(lexer.lex().tokens, expected);
    }
}
