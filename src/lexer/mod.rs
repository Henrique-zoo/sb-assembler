//! Analisador léxico da linguagem assembly.
//!
//! Este módulo transforma o texto-fonte em uma sequência de `Token`s com `Span`,
//! preservando posição (linha/coluna/offset) para diagnóstico de erros.
//!
//! Regras gerais:
//! - espaços horizontais são ignorados;
//! - `\n` é emitido como `TokenKind::NewLine`;
//! - comentários iniciados por `;` são ignorados até o fim da linha;
//! - no fim da entrada, `TokenKind::Eof` é emitido uma única vez.
//!
//! Literais numéricos aceitos: decimal, `0x`/`0X` (hex) e `0b`/`0B` (bin).

mod types;

use crate::{
    errors::{LexerError, LexerErrorKind},
    interner::Interner,
};

pub(crate) use types::{Span, Token, TokenKind};

/// Cursor lógico no arquivo fonte.
///
/// `offset` representa um índice em bytes dentro de `src`.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    /// Posição absoluta em bytes na string fonte.
    offset: usize,
    /// Linha atual (1-based).
    line: u32,
    /// Coluna atual (1-based).
    column: u32,
}

impl Cursor {
    /// Cria um cursor no início lógico do arquivo.
    /// Estado inicial:
    /// - `offset = 0` (primeiro byte da string fonte)
    /// - `line = 1`
    /// - `column = 1`
    /// Não depende de leitura prévia nem altera nenhum estado externo.
    fn new() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }
}

/// Lexer do assembly.
///
/// Lifetimes:
/// - `'src`: lifetime do código-fonte.
/// - `'interner`: lifetime do interner.
///
/// Eles são separados porque fonte e interner não precisam,
/// necessariamente, compartilhar o mesmo lifetime.
#[derive(Debug)]
pub(super) struct Lexer<'src, 'interner> {
    /// Código-fonte completo.
    src: &'src str,
    /// Interner usado para deduplicar lexemas.
    interner: &'interner mut Interner,

    /// Posição lógica atual de leitura.
    cursor: Cursor,

    /// Caractere atual "espiado", sem consumo.
    peeked: Option<(usize, char)>,

    /// Iterador Unicode-safe sobre os caracteres da fonte.
    chars: std::str::CharIndices<'src>,

    /// Indica se o token EOF já foi emitido.
    emitted_eof: bool,
}

impl<'src, 'interner> Lexer<'src, 'interner> {
    /// Inicializa o lexer com a fonte e o interner compartilhado.
    ///
    /// Como o estado é inicializado:
    /// - `cursor` começa no início do arquivo (`Cursor::new()`).
    /// - `chars` recebe `src.char_indices()` para iteração Unicode-safe.
    /// - `peeked` já guarda o primeiro caractere (se existir), permitindo
    ///   olhar o caractere atual sem consumir.
    /// - `emitted_eof` começa em `false`, para que o token EOF seja emitido
    ///   exatamente uma vez.
    pub(crate) fn new(src: &'src str, interner: &'interner mut Interner) -> Self {
        let mut chars = src.char_indices();
        let peeked = chars.next();

        Self {
            src,
            interner,
            cursor: Cursor::new(),
            peeked,
            chars,
            emitted_eof: false,
        }
    }

    /// Constrói um `Span` entre um cursor inicial já salvo e um `end_offset`.
    ///
    /// Não altera estado do lexer; apenas empacota:
    /// - posição/linha/coluna de `start`
    /// - comprimento calculado como `end_offset - start.offset` (com proteção
    ///   contra underflow via `saturating_sub`).
    fn current_span(&self, start: Cursor, end_offset: usize) -> Span {
        Span {
            pos: start.offset,
            line: start.line,
            column: start.column,
            len: end_offset.saturating_sub(start.offset),
        }
    }

    /// Gera o `Span` de EOF na posição atual do cursor.
    ///
    /// Não altera estado do lexer. O span sempre tem `len = 0` e aponta
    /// exatamente para o fim lógico da leitura.
    fn eof_span(&self) -> Span {
        Span {
            pos: self.cursor.offset,
            line: self.cursor.line,
            column: self.cursor.column,
            len: 0,
        }
    }

    /// Retorna o caractere atual sem consumir entrada.
    ///
    /// Aqui "peek" lê `self.peeked`, que já contém o próximo item pronto.
    /// Diferente de chamar `self.chars.next()`, essa operação não avança
    /// iterador nem altera cursor.
    #[inline]
    fn peek(&self) -> Option<char> {
        self.peeked.map(|(_, ch)| ch)
    }

    /// Consome o caractere atual e avança o estado do lexer.
    ///
    /// Efeitos no estado:
    /// - Usa `self.peeked` como caractere corrente consumido.
    /// - Atualiza `cursor.offset` para o byte imediatamente após o caractere.
    /// - Se o caractere for `\n`:
    ///   - incrementa `cursor.line`
    ///   - reinicia `cursor.column` para `1`
    /// - Caso contrário:
    ///   - incrementa `cursor.column` em `1`
    /// - Atualiza `self.peeked` com o próximo caractere de `self.chars`.
    ///
    /// Retorna `(offset_inicial, caractere)` consumido, ou `None` em EOF.
    fn bump(&mut self) -> Option<(usize, char)> {
        let (start_offset, ch) = self.peeked?;

        let ch_len = ch.len_utf8();
        self.cursor.offset = start_offset + ch_len;

        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 1;
        } else {
            self.cursor.column += 1;
        }

        self.peeked = self.chars.next();
        Some((start_offset, ch))
    }

    /// Consome whitespace horizontal (`' '`, `'\t'`, `'\r'`), sem tocar em `\n`.
    ///
    /// Efeito no estado:
    /// - Repetidamente chama `bump()`, então avança `offset/column`
    ///   (e eventualmente linha/coluna no caso de `\r` não muda linha).
    /// - Para assim que encontra algo que não seja whitespace horizontal
    ///   ou quando chega em EOF.
    fn skip_horizontal_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
            let _ = self.bump();
        }
    }

    /// Consome o conteúdo de comentário iniciado por `;` até antes de `\n` ou EOF.
    ///
    /// Efeito no estado:
    /// - Avança `cursor` chamando `bump()` para cada caractere do comentário.
    /// - Não consome o `\n`; esse caractere fica para ser tokenizado depois
    ///   como `TokenKind::NewLine`.
    /// - Aceita Unicode normalmente, pois a iteração é feita por `char`.
    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            let _ = self.bump();
        }
    }

    /// Lê e produz um token de nova linha.
    ///
    /// Efeito no estado:
    /// - Salva o cursor inicial.
    /// - Consome exatamente um `\n` com `bump()`, o que incrementa a linha e
    ///   reinicia a coluna para `1`.
    /// - Retorna `TokenKind::NewLine` com span cobrindo esse caractere.
    fn lex_newline(&mut self) -> Token {
        let start = self.cursor;
        let _ = self.bump();
        let span = self.current_span(start, self.cursor.offset);
        Token::new(TokenKind::NewLine, span)
    }

    /// Lê e produz um token de caractere único (`,`, `:`, `+`, `-`, etc.).
    ///
    /// Efeito no estado:
    /// - Salva o cursor inicial.
    /// - Consome exatamente um caractere com `bump()`.
    /// - Retorna token do `kind` informado com span de um caractere.
    fn lex_single_char(&mut self, kind: TokenKind) -> Token {
        let start = self.cursor;
        let _ = self.bump();
        let span = self.current_span(start, self.cursor.offset);
        Token::new(kind, span)
    }

    /// Lê um identificador ASCII (`[A-Za-z_][A-Za-z0-9_]*`).
    ///
    /// Efeito no estado:
    /// - Consome todos os caracteres válidos do identificador via `bump()`.
    /// - Atualiza `cursor` até o primeiro caractere inválido (que não é consumido).
    /// - Extrai o slice correspondente em `src` e registra no `interner`,
    ///   podendo inserir um novo símbolo.
    /// - Retorna `TokenKind::Ident(sym)` com span do lexema.
    fn lex_ident(&mut self) -> Token {
        let start = self.cursor;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                let _ = self.bump();
            } else {
                break;
            }
        }

        let text = &self.src[start.offset..self.cursor.offset];
        let sym = self.interner.entry(text).or_insert();
        let span = self.current_span(start, self.cursor.offset);

        Token::new(TokenKind::Ident(sym), span)
    }

    /// Lê um número inteiro em base decimal, hexadecimal ou binária.
    ///
    /// Efeito no estado:
    /// - Sempre consome o primeiro dígito (`[0-9]`) que disparou o lexer.
    /// - Se o número começar com `0x`/`0X`, passa a consumir dígitos hexadecimais.
    /// - Se o número começar com `0b`/`0B`, passa a consumir dígitos binários.
    /// - Sem prefixo, consome os dígitos decimais restantes.
    /// - Para no primeiro caractere que não pertence à base atual.
    /// - Interna o texto do número no `interner`.
    /// - Retorna `TokenKind::Number(sym)` com span do lexema.
    ///
    /// Retorna erro para prefixos sem dígitos (`0x`/`0b` sem payload).
    fn lex_number(&mut self) -> Result<Token, LexerError> {
        enum NumberBase {
            Dec,
            Hex,
            Bin,
        }

        let mut base = NumberBase::Dec;
        let start = self.cursor;
        let (_, first) = self.bump().expect("lex_number starts on a digit");

        if first == '0' {
            match self.peek() {
                Some('x' | 'X') => {
                    let _ = self.bump();
                    base = NumberBase::Hex;
                }
                Some('b' | 'B') => {
                    let _ = self.bump();
                    base = NumberBase::Bin;
                }
                _ => {}
            }
        }

        let mut empty_after_base_digit = true;
        while let Some(ch) = self.peek() {
            let valid = match base {
                NumberBase::Dec => ch.is_ascii_digit(),
                NumberBase::Hex => ch.is_ascii_hexdigit(),
                NumberBase::Bin => matches!(ch, '0' | '1'),
            };

            if valid {
                let _ = self.bump();
                empty_after_base_digit = false;
            } else {
                break;
            }
        }

        if matches!(base, NumberBase::Hex | NumberBase::Bin) && empty_after_base_digit {
            return Err(self.invalid_number_error(start));
        }

        let text = &self.src[start.offset..self.cursor.offset];
        let sym = self.interner.entry(text).or_insert();
        let span = self.current_span(start, self.cursor.offset);

        Ok(Token::new(TokenKind::Number(sym), span))
    }

    /// Cria erro léxico para literal numérico malformado.
    ///
    /// Não altera o estado do lexer; apenas descreve o literal já consumido
    /// entre `start` e o cursor atual.
    fn invalid_number_error(&self, start: Cursor) -> LexerError {
        let span = self.current_span(start, self.cursor.offset);
        let text = self.src[start.offset..self.cursor.offset].to_string();

        LexerError {
            kind: LexerErrorKind::InvalidNumber(text),
            span,
        }
    }

    /// Cria erro léxico para caractere inválido na posição atual do cursor.
    ///
    /// Efeito no estado:
    /// - Consome o caractere inválido com `bump()` para permitir que iteração
    ///   continue após o erro (evita repetir o mesmo erro infinitamente).
    ///
    /// O erro retornado aponta para a posição original do caractere inválido:
    /// - `kind = InvalidChar(ch)`
    /// - `span` no cursor antes do consumo (`offset/line/column`)
    /// - `len` em bytes do caractere inválido
    fn invalid_char_error(&mut self, ch: char) -> LexerError {
        let start = self.cursor;
        let _ = self.bump();

        LexerError {
            kind: LexerErrorKind::InvalidChar(ch),
            span: Span {
                pos: start.offset,
                line: start.line,
                column: start.column,
                len: ch.len_utf8(),
            },
        }
    }
}

impl<'src, 'intern> Iterator for Lexer<'src, 'intern> {
    type Item = Result<Token, LexerError>;

    /// Produz o próximo token do fluxo léxico.
    ///
    /// Fluxo e efeitos de estado:
    /// - Se não houver mais caractere em `peeked`:
    ///   - emite EOF uma única vez (`emitted_eof = true`) e depois retorna `None`.
    /// - Antes de tokenizar, consome whitespace horizontal com
    ///   `skip_horizontal_whitespace()`.
    /// - Reconhece e consome comentários iniciados por `;` com `skip_comment()`,
    ///   voltando ao loop sem emitir token.
    /// - Para caracteres válidos, delega para os lexers específicos, que avançam
    ///   `cursor` e retornam o token correspondente.
    /// - Para caractere não reconhecido, retorna `Err` e avança 1 caractere
    ///   para evitar repetição infinita do mesmo erro em iterações seguintes.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.peek().is_none() {
                if self.emitted_eof {
                    return None;
                }

                self.emitted_eof = true;
                return Some(Ok(Token::new(TokenKind::Eof, self.eof_span())));
            }

            self.skip_horizontal_whitespace();

            if self.peek().is_none() {
                continue;
            }

            let ch = self.peek().expect("peek checked above");

            return match ch {
                ';' => {
                    self.skip_comment();
                    continue;
                }
                '\n' => Some(Ok(self.lex_newline())),
                '&' => Some(Ok(self.lex_single_char(TokenKind::Ampersand))),
                ',' => Some(Ok(self.lex_single_char(TokenKind::Comma))),
                ':' => Some(Ok(self.lex_single_char(TokenKind::Colon))),
                '+' => Some(Ok(self.lex_single_char(TokenKind::Plus))),
                '-' => Some(Ok(self.lex_single_char(TokenKind::Minus))),

                '0'..='9' => Some(self.lex_number()),

                'A'..='Z' | 'a'..='z' | '_' => Some(Ok(self.lex_ident())),

                other => Some(Err(self.invalid_char_error(other))),
            };
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
