#![feature(str_char)]
#![feature(test)]

extern crate test;

mod lexer;
mod sanitizer;
mod writer;

#[derive(Debug,PartialEq)]
pub enum Keyword {
    Select,  // SELECT
    From,    // FROM
    Where,   // WHERE
    And,     // AND
    Or,      // OR
    Update,  // UPDATE
    Set,     // SET
    Insert,  // INSERT
    Into,    // INTO
    Values,  // VALUES
    Inner,   // INNER
    Join,    // JOIN
    On,      // ON
    Limit,   // LIMIT
    Offset,  // OFFSET
    Between, // BETWEEN
    Other(BufferPosition)
}

#[derive(Debug,PartialEq)]
pub enum Operator {
    Arithmetic(ArithmeticOperator),
    Logical(LogicalOperator),
    Comparison(ComparisonOperator),
    Bitwise(BitwiseOperator)
}

#[derive(Debug,PartialEq)]
pub enum ArithmeticOperator {
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    Plus,     // +
    Minus     // -
}

#[derive(Debug,PartialEq)]
pub enum LogicalOperator {
    In,    // IN
    Not,   // NOT
    Like,  // LIKE
    Rlike, // RLIKE
    Glob,  // GLOB
    Match, // MATCH
    Regexp // REGEXP
}

#[derive(Debug,PartialEq)]
pub enum ComparisonOperator {
    Equal,              // =
    Equal2,             // ==
    NullSafeEqual,      // <=>
    GreaterThanOrEqual, // =>
    LessThanOrEqual,    // <=
    EqualOrGreaterThan, // =>
    EqualOrLessThan,    // <=
    EqualWithArrows,    // <>
    NotEqual,           // !=
    GreaterThan,        // >
    LessThan            // <
}

#[derive(Debug,PartialEq)]
pub enum BitwiseOperator {
    LeftShift,  // <<
    RightShift, // >>
    And,        // &
    Or          // |
}

#[derive(Debug,PartialEq)]
pub struct BufferPosition {
    pub start: usize,
    pub end: usize
}

impl BufferPosition {
    pub fn new(start: usize, end: usize) -> BufferPosition {
        assert!(start < end);
        BufferPosition {
            start: start,
            end: end
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum Token {
    Operator(Operator),
    Keyword(Keyword),
    Backticked(BufferPosition),
    DoubleQuoted(BufferPosition),
    SingleQuoted(BufferPosition),
    Numeric(BufferPosition),
    Space,
    Newline,
    Dot,
    Comma,
    Wildcard,
    ParentheseOpen,
    ParentheseClose,
    Colon,
    Semicolon,
    Placeholder,
    NumberedPlaceholder(BufferPosition),
    Unknown(char)
}

#[derive(Debug,PartialEq)]
pub struct Sql {
    buf: String,
    pub tokens: Vec<Token>
}

impl Sql {
    pub fn buffer_content(&self, pos: &BufferPosition) -> &str {
        &self.buf[pos.start..pos.end]
    }
}

/// Lex a sql string into a `Sql` struct that contains the original
/// buffer and the tokens found.
pub fn lex(buf: String) -> Sql {
    lexer::SqlLexer::new(buf).lex()
}

/// Write a `Sql` struct back to a sql string.
pub fn write(sql: Sql) -> String {
    writer::SqlWriter::new(sql).write()
}

/// Sanitize a `Sql` struct
pub fn sanitize(sql: Sql) -> Sql {
    sanitizer::SqlSanitizer::new(sql).sanitize()
}

/// Returns a sanitized sql string
pub fn sanitize_string(buf: String) -> String {
    write(sanitize(lex(buf)))
}

#[cfg(test)]
mod tests {
    use test;
    use super::Sql;
    use super::{Token,Operator,BufferPosition,Keyword,ComparisonOperator};

    #[test]
    fn test_buffer_content() {
        let sql = Sql {
            buf: "SELECT `table`.* FROM `table` WHERE `id` = 'secret';".to_string(),
            tokens: Vec::new()
        };
        let buffer_position = BufferPosition::new(17, 21);

        assert_eq!("FROM", sql.buffer_content(&buffer_position));
    }

    #[test]
    fn test_lex() {
        let sql_buffer = "SELECT * FROM `table`";

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Backticked(BufferPosition::new(15, 20))
        ];

        let sql = super::lex(sql_buffer.to_string());
        assert_eq!(sql.buf, sql_buffer);
        assert_eq!(sql.tokens, expected);
    }

    #[test]
    fn test_write() {
        let sql_buffer = "SELECT * FROM `table`";
        assert_eq!(super::write(super::lex(sql_buffer.to_string())), sql_buffer);
    }

    #[test]
    fn test_sanitize() {
        let sql = super::sanitize(super::lex("SELECT * FROM `table` WHERE `id` = 1;".to_string()));

        let expected = vec![
            Token::Keyword(Keyword::Select),
            Token::Space,
            Token::Wildcard,
            Token::Space,
            Token::Keyword(Keyword::From),
            Token::Space,
            Token::Backticked(BufferPosition::new(15, 20)),
            Token::Space,
            Token::Keyword(Keyword::Where),
            Token::Space,
            Token::Backticked(BufferPosition::new(29, 31)),
            Token::Space,
            Token::Operator(Operator::Comparison(ComparisonOperator::Equal)),
            Token::Space,
            Token::Placeholder,
            Token::Semicolon
        ];

        assert_eq!(sql.tokens, expected);
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(
            super::sanitize_string("SELECT * FROM `table` WHERE id = 1;".to_string()),
            "SELECT * FROM `table` WHERE id = ?;"
        );
    }

    #[bench]
    fn bench_sanitize_string_quote(b: &mut test::Bencher) {
        b.iter(|| {
            test::black_box(super::sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 'secret' LIMIT 1;".to_string()));
        });
    }

    #[bench]
    fn bench_sanitize_string_numeric(b: &mut test::Bencher) {
        b.iter(|| {
            test::black_box(super::sanitize_string("SELECT `table`.* FROM `table` WHERE `id` = 1 LIMIT 1;".to_string()));
        });
    }

    #[bench]
    fn bench_sanitize_string_in(b: &mut test::Bencher) {
        b.iter(|| {
            test::black_box(super::sanitize_string("SELECT `table`.* FROM `table` WHERE `id` IN (1, 2, 3) LIMIT 1;".to_string()));
        });
    }

    #[bench]
    fn bench_sanitize_insert(b: &mut test::Bencher) {
        b.iter(|| {
            test::black_box(super::sanitize_string("INSERT INTO \"table\" (\"field1\", \"field2\") VALUES ('value', 1);".to_string()));
        });
    }
}
