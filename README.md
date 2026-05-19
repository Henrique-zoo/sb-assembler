# sb-assembler

`sb-assembler` é um montador, pré-processador e simulador para uma linguagem
assembly didática de 16 bits. O projeto implementa o pipeline usado em
trabalhos de Software Básico: a partir de um arquivo `.asm`, gera um arquivo
pré-processado `.pre`; a partir do `.pre`, gera os artefatos `.obj` e `.pen`;
e, a partir de um `.obj`, executa o programa em uma máquina simulada.

## Uso

Execute o binário passando um único arquivo de entrada:

```sh
cargo run --release -- <caminho/do/programa.ext>
```
Alternativamente, compile antes e execute a partir do binário:
```sh
cargo build --release
./sb-assembler <caminho/do/programa.ext>
```
A flag `--release` é opcional. Sua utilização ativa otimizações do compilador, deixando o programa mais rápido.

O modo de operação é escolhido pela extensão do arquivo:

| Entrada | Ação | Saída |
| --- | --- | --- |
| `.asm` | Executa lexer e pré-processador | gera `.pre` |
| `.pre` | Executa lexer, pré-processador, parser e montagem | gera `.obj` e `.pen` |
| `.obj` | Executa o simulador | escreve a saída do programa no stdout |

Arquivos com outras extensões são rejeitados.

## Funcionalidades

- Análise léxica com comentários iniciados por `;`.
- Pré-processamento com `SECTION TEXT`, `SECTION DATA`, `MACRO`, `ENDMACRO`,
  `EQU` e `IF`.
- Parser para instruções de `TEXT` e diretivas montáveis de `DATA`.
- Montagem em uma passagem, gerando:
  - `.obj`: programa objeto resolvido;
  - `.pen`: programa com listas de pendência da montagem em uma passagem.
- Simulador independente do assembler, capaz de executar arquivos `.obj`.
- Diagnósticos por estágio: lexer, pré-processador, parser, montagem e
  simulação.

## Requisitos

- Rust com Cargo.

O projeto não possui dependências externas além da biblioteca padrão.

## Exemplo

Arquivo `exemplo.asm`:

```asm
SECTION DATA
VALUE: CONST 42

SECTION TEXT
OUTPUT VALUE
STOP
```

Gerar o pré-processado:

```sh
cargo run -- exemplo.asm
```

Resultado em `exemplo.pre`:

```asm
SECTION TEXT
OUTPUT VALUE
STOP
SECTION DATA
VALUE: CONST 42
```

Gerar objeto e pendências:

```sh
cargo run -- exemplo.pre
```

Resultado em `exemplo.obj`:

```text
13 0 3 0 14 0 42 0
```

Resultado em `exemplo.pen`:

```text
13 0 14 42
```

Simular o objeto:

```sh
cargo run -- exemplo.obj
```

Saída:

```text
42
```

## Linguagem suportada

O fonte é normalizado para caixa alta antes do processamento. Assim, `load`,
`Load` e `LOAD` são tratados da mesma forma.

### Seções

O programa montável é separado em duas seções:

```asm
SECTION TEXT
; instruções

SECTION DATA
; dados
```

Na montagem, `TEXT` é emitida antes de `DATA`. Rótulos definidos em qualquer
seção podem ser usados como operandos de endereço.

### Instruções

| Instrução | Operandos |
| --- | --- |
| `ADD` | 1 endereço |
| `SUB` | 1 endereço |
| `MULT` | 1 endereço |
| `DIV` | 1 endereço |
| `JMP` | 1 endereço |
| `JMPN` | 1 endereço |
| `JMPP` | 1 endereço |
| `JMPZ` | 1 endereço |
| `COPY` | 2 endereços |
| `LOAD` | 1 endereço |
| `STORE` | 1 endereço |
| `INPUT` | 1 endereço |
| `OUTPUT` | 1 endereço |
| `STOP` | nenhum |

Operandos de endereço aceitam rótulos diretos e deslocamentos:

```asm
LOAD VALUE
LOAD TABLE + 2
STORE TABLE - 1
COPY SRC, DST
```

### Diretivas de dados

```asm
VALUE: CONST 10
NEGATIVE: CONST -1
BUFFER: SPACE
TABLE: SPACE 8
```

- `CONST` emite uma palavra de 16 bits.
- `SPACE` reserva palavras zeradas; sem quantidade, reserva uma palavra.

### Pré-processamento

`EQU` define aliases numéricos ou aliases de aliases:

```asm
ONE EQU 1
TRUE EQU ONE
```

`IF` consome a próxima linha lógica. Se a condição for zero, a linha controlada
não é emitida:

```asm
ENABLE EQU 1

SECTION TEXT
IF ENABLE
OUTPUT VALUE
```

Macros são declaradas com parâmetros prefixados por `&`:

```asm
ACCUM: MACRO &DST, &SRC, &TMP
LOAD &TMP
ADD &SRC
STORE &DST
ENDMACRO

SECTION TEXT
ACCUM RESULT, INPUT_VALUE, AUX
```

## Formato do `.obj`

O arquivo `.obj` é textual. Cada token separado por espaço representa um byte
decimal entre `0` e `255`. A cada dois bytes, o simulador reconstrói uma palavra
de 16 bits em little-endian:

```text
10 0 7 0 14 0
```

A quantidade de bytes precisa ser par. Um token fora do intervalo de byte ou um
byte solto no final do arquivo produz erro de simulação.

## Simulador

O simulador é um módulo próprio, separado da fachada do assembler. Ele recebe um
programa objeto já resolvido ou um arquivo `.obj`, carrega as palavras em uma
memória de 65.536 posições e executa a partir do endereço `0` até encontrar
`STOP`.

A memória não carrega metadados de seção. O `.obj` contém `TEXT` seguido de
`DATA`, e apenas o fluxo do contador de programa decide o que será executado.

O acumulador é assinado de 16 bits. Operações aritméticas usam comportamento
circular para simular uma máquina real de 16 bits: overflow não causa `panic`;
os bits resultantes permanecem no registrador.

## Arquitetura

Principais módulos:

- `lexer`: transforma texto em tokens e preserva spans para diagnósticos.
- `preprocessor`: processa seções, macros, `EQU` e `IF`.
- `parser`: valida a linguagem montável depois do pré-processamento.
- `assembly`: emite `.obj` e `.pen` com montagem em uma passagem.
- `assembler`: fachada de alto nível do pipeline de montagem.
- `simulator`: carrega e executa programas objeto.
- `errors`: tipos de erro e famílias de diagnósticos.

O binário em `src/main.rs` apenas escolhe o modo pela extensão do arquivo e
chama o módulo apropriado.

## Testes

Rodar a suíte completa:

```sh
cargo test
```

Checar formatação:

```sh
cargo fmt --check
```

Checar compilação de todos os targets:

```sh
cargo check --all-targets
```

Os testes de integração ficam em `tests/` e cobrem o pipeline do assembler, o
pré-processador, o simulador e os modos do CLI.

## Licença

Este projeto está licenciado sob os termos da licença MIT. Veja `LICENSE`.
