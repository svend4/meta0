#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Keyword(String),
    Identifier(String),
    StringLiteral(String),
    Number(f64),
    Operator(String),
    Punctuation(char),
    Eof,
}

pub struct Lexer {
    input: String,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.to_string(),
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let c = chars[i];
            
            if c.is_whitespace() {
                i += 1;
                continue;
            }
            
            if c == '/' && i + 1 < chars.len() && chars[i+1] == '/' {
                while i < chars.len() && chars[i] != '\n' { i += 1; }
                continue;
            }
            
            if c == '{' || c == '}' || c == '[' || c == ']' || c == '(' || c == ')' || c == ':' || c == ',' {
                tokens.push(Token::Punctuation(c));
                i += 1;
                continue;
            }
            
            if c == '"' {
                i += 1;
                let mut string_val = String::new();
                while i < chars.len() && chars[i] != '"' {
                    string_val.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() { i += 1; } // Skip closing quote
                tokens.push(Token::StringLiteral(string_val));
                continue;
            }
            
            if c == '=' && i + 1 < chars.len() && chars[i+1] == '>' {
                tokens.push(Token::Operator("=>".to_string()));
                i += 2;
                continue;
            }
            
            if c == '>' && i + 1 < chars.len() && chars[i+1] == '=' {
                tokens.push(Token::Operator(">=".to_string()));
                i += 2;
                continue;
            }
            
            if c == '<' && i + 1 < chars.len() && chars[i+1] == '=' {
                tokens.push(Token::Operator("<=".to_string()));
                i += 2;
                continue;
            }
            
            if c == '=' && i + 1 < chars.len() && chars[i+1] == '=' {
                tokens.push(Token::Operator("==".to_string()));
                i += 2;
                continue;
            }
            
            if c == '!' && i + 1 < chars.len() && chars[i+1] == '=' {
                tokens.push(Token::Operator("!=".to_string()));
                i += 2;
                continue;
            }
            
            if c == '>' || c == '<' || c == '=' {
                tokens.push(Token::Operator(c.to_string()));
                i += 1;
                continue;
            }
            
            // identifier or keyword or number
            let mut word = String::new();
            while i < chars.len() && !chars[i].is_whitespace() && !"{}[](:),\"".contains(chars[i]) {
                word.push(chars[i]);
                i += 1;
            }
            
            match word.as_str() {
                "MODULE" | "QUANTUM_STATE" | "STORE" | "ADD" | "SUB" | "MUL" | "DIV" | "IF" | "ELSE" | "LOOP" | "ORACLE_FETCH" | "AI_ANALYZE" | "INTUITION_BRANCH" | "TELEPORT" | "PAYLOAD" | "EXTRACT" | "AS" | "MINT_TOKEN" | "TRANSFER_TOKEN" | "TO" => tokens.push(Token::Keyword(word)),
                _ => {
                    if let Ok(num) = word.parse::<f64>() {
                        tokens.push(Token::Number(num));
                    } else {
                        tokens.push(Token::Identifier(word));
                    }
                }
            }
        }
        tokens.push(Token::Eof);
        tokens
    }
}
