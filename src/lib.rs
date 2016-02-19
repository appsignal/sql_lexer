#![feature(str_char)]

#[macro_use] extern crate lazy_static;
extern crate regex;

use regex::Regex;

lazy_static! {
    static ref WORD: Regex = Regex::new("^\\w+").unwrap();
    static ref BACKTICKED: Regex = Regex::new("^`(.*?)`").unwrap();
    static ref SINGLE_QUOTED: Regex = Regex::new("^'(.*?|[^'])'").unwrap();
}

#[derive(Debug,PartialEq)]
pub enum Operator {
    Dot,
    Multiply,
    Equals
}

#[derive(Debug,PartialEq)]
pub enum Token {
    Keyword(String),
    Operator(Operator),
    Grouped(String),
    DoubleQuoted(String),
    SingleQuoted(String),
    Numeric(String),
    Backticked(String),
    Space
}

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

    fn find_with_regex(&self, regex: &Regex) -> Option<(usize, usize)> {
        match regex.find(&self.buf[self.pos..]) {
            Some((s, e)) => Some((s + self.pos, e + self.pos)),
            None => None
        }
    }
}

impl Iterator for SqlLexer {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.len {
            return None;
        }

        match self.buf.char_at(self.pos) {
            c if c.is_alphabetic() => {
                let (start, end) = match self.find_with_regex(&WORD) {
                    Some((s, e)) => (s, e),
                    None => return None
                };
                self.pos = end;
                Some(Token::Keyword(self.buf[start..end].to_string()))
            },
            '`' => {
                let (start, end) = match self.find_with_regex(&BACKTICKED) {
                    Some((s, e)) => (s, e),
                    None => return None
                };
                self.pos = end;
                Some(Token::Backticked(self.buf[start + 1..end - 1].to_string()))
            },
            '\'' => {
                let (start, end) = match self.find_with_regex(&SINGLE_QUOTED) {
                    Some((s, e)) => (s, e),
                    None => return None
                };
                self.pos = end;
                Some(Token::SingleQuoted(self.buf[start + 1..end - 1].to_string()))
            },
            '.' => {
                self.pos += 1;
                Some(Token::Operator(Operator::Dot))
            },
            '*' => {
                self.pos += 1;
                Some(Token::Operator(Operator::Multiply))
            },
            '=' => {
                self.pos += 1;
                Some(Token::Operator(Operator::Equals))
            },
            ' ' => {
                self.pos += 1;
                Some(Token::Space)
            },
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SqlLexer,Token,Operator};

    #[test]
    fn test_lexing_single_quoted() {
        let sql = "SELECT `table`.* FROM `table` WHERE `id` = 'secret'".to_string();

        let expected = vec![
            Token::Keyword("SELECT".to_string()),
            Token::Space,
            Token::Backticked("table".to_string()),
            Token::Operator(Operator::Dot),
            Token::Operator(Operator::Multiply),
            Token::Space,
            Token::Keyword("FROM".to_string()),
            Token::Space,
            Token::Backticked("table".to_string()),
            Token::Space,
            Token::Keyword("WHERE".to_string()),
            Token::Space,
            Token::Backticked("id".to_string()),
            Token::Space,
            Token::Operator(Operator::Equals),
            Token::Space,
            Token::Keyword("LIMIT".to_string()),
            Token::Space,
            Token::Numeric("1".to_string())
        ];

        let lexer = SqlLexer::new(sql);
        assert_eq!(lexer.collect::<Vec<Token>>(), expected);
    }
}
