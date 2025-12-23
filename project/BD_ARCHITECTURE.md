## Arquitetura de Banco de Dados - Organização e Uso

### Visão Geral

A camada de banco de dados foi reorganizada para:

1. **Separar responsabilidades**: Contexto de conexão vs. Lógica de consultas
2. **Manter uma única instância aberta**: Usando global `DbContext` com `Arc` para compartilhamento thread-safe
3. **Diferenciar leitura e escrita**: Read-only e write connections separadas

### Estrutura de Arquivos

```
src/bd/
├── mod.rs                    # Exports públicos e re-exports
├── init.rs                   # Inicialização e getter global
├── connection.rs             # DbContext (containers de conexões)
└── repository/
    ├── mod.rs
    └── ledger.rs             # LedgerRepository (queries e mutations)
```

### Como Usar

#### 1. Inicializar o Banco de Dados (uma única vez na startup)

```rust
use bd::init_db;

fn main() {
    init_db(None).expect("Failed to initialize database");
    // ... resto da aplicação
}
```

#### 2. Acessar o Banco em Qualquer Lugar

```rust
use bd::get_db;

fn example() {
    let db = get_db();  // Retorna Arc<DbContext>
    let ledger = db.ledger();  // Obtém o repository

    // Ler dados (read-only)
    let utxos = ledger.get_utxos_for_address("addr").unwrap();

    // Escrever dados
    ledger.apply_block(block, &transactions).unwrap();
}
```

### Componentes Principais

#### `DbContext` (src/bd/connection.rs)

- **Responsabilidade**: Gerenciar as conexões SQLite
- **Campo write_conn**: Conexão de escrita (transações)
- **Campo read_conn**: Conexão de leitura (queries isoladas)
- **Métodos principais**:
  - `open()`: Abre ou cria banco (arquivo ou `:memory:`)
  - `init_schema()`: Cria tabelas se não existirem
  - `ledger()`: Retorna um `LedgerRepository` para queries

#### `LedgerRepository` (src/bd/repository/ledger.rs)

- **Responsabilidade**: Encapsular toda lógica de consultas ao banco
- **Métodos de leitura**: `get_utxos_for_address()`, `get_transaction()`, etc.
- **Métodos de escrita**: `apply_block()`, `insert_mempool_tx()`, etc.
- **Benefício**: Node e Wallet não precisam conhecer SQL ou details SQLite

#### `init_db` / `get_db` (src/bd/init.rs)

- **init_db()**: Abre o contexto uma vez e o armazena globalmente
- **get_db()**: Retorna `Arc<DbContext>` já inicializado
- **Por que Arc?**: SQLite `Connection` não é `Sync`, mas `unsafe impl` permite compartilhamento

### Comparação: Antes vs. Depois

**Antes (Problema)**:

```rust
// Abrindo banco toda vez que precisa
let db = Db::open(None).unwrap();
db.get_utxos_for_address("addr");  // Múltiplas aberturas = overhead
```

**Depois (Solução)**:

```rust
// Uma única abertura na inicialização
init_db(None);

// Em qualquer lugar da app:
let db = get_db();  // Retorna a instância já aberta (via Arc)
db.ledger().get_utxos_for_address("addr");
```

### Testes

Os testes usam `create_test_db()` para criar instâncias isoladas (útil para paralelização):

```rust
#[test]
fn test_example() {
    let db = create_test_db(Some(":memory:")).unwrap();
    let ledger = db.ledger();
    // ...
}
```

### Benefícios Finais

✅ **Performance**: Uma única conexão aberta  
✅ **Limpeza**: Lógica de queries isolada em `LedgerRepository`  
✅ **Segurança**: Read/Write connections explícitas  
✅ **Testabilidade**: Testes criam instâncias isoladas  
✅ **Manutenção**: Node/Wallet não acoplados a SQLite details
